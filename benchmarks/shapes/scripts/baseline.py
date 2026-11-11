"""Independent build123d 0.11.1 construction from shared IR, evaluation only.
No Rust-produced STEP/mesh is imported. Supported subset is exactly the six fixtures.
"""
import time
started=time.perf_counter()
import build123d as bd
import json,sys,copy,resource
loaded=time.perf_counter()
m=json.load(sys.stdin);p=m['parameters'];bodies={};nets={}
def s(v):
    if isinstance(v,(int,float)):return v
    if 'param' in v:return p[v['param']]
    a,b=s(v['a']),s(v['b']);return {'add':lambda:a+b,'sub':lambda:a-b,'mul':lambda:a*b,'div':lambda:a/b}[v['op']]()
def vec(v):return tuple(s(x) for x in v)
def prism(profile,h,inset=0):
    if profile['kind']=='rectangle':
        w,d=s(profile['width'])-2*inset,s(profile['depth'])-2*inset;r=max(0,s(profile.get('radius',0))-inset)
        sketch=bd.RectangleRounded(w,d,r) if r else bd.Rectangle(w,d)
    elif profile['kind']=='circle':sketch=bd.Circle(s(profile['radius'])-inset)
    else:raise ValueError('unsupported prism profile')
    return bd.extrude(sketch,amount=h)
def frame(b,origin,x,n):return b.moved(bd.Plane(origin=origin,x_dir=x,z_dir=n).location)
def face_frame(b,face,c):
    bb=b.bounding_box();u,v=c;e=1e-4
    return {'neg_y':((u,bb.min.Y-e,v),(-1,0,0),(0,1,0)),
            'pos_y':((u,bb.max.Y+e,v),(1,0,0),(0,-1,0))}[face]
def union(a,b):
    r=a.fuse(*b.solids())
    return bd.Compound(list(r)) if isinstance(r,bd.ShapeList) else r
for f in m['features']:
    op=f['op']
    if op=='box':b=bd.Box(*vec(f['size']),align=(bd.Align.MIN,)*3).translate(vec(f['origin']))
    elif op=='hole_grid':
        b=bodies[f['input']];o=vec(f['origin']);tools=[]
        for y in range(int(s(f['count'][1]))):
            for x in range(int(s(f['count'][0]))):
                tools.append(bd.Cylinder(s(f['radius']),s(f['depth']),align=(bd.Align.CENTER,bd.Align.CENTER,bd.Align.MIN)).translate((o[0]+x*s(f['spacing'][0]),o[1]+y*s(f['spacing'][1]),o[2])))
        # Native batch cut: avoid deliberately choosing a slow Python per-hole loop.
        b=b.cut(*tools)
    elif op=='fillet':
        b=bodies[f['input']];sel=f['select'];axis='xyz'.index(sel['axis']);value=s(sel['value']);tol=s(sel['tolerance'])
        edges=[e for e in b.edges() if abs(list(e.bounding_box().min)[axis]-value)<=tol and abs(list(e.bounding_box().max)[axis]-value)<=tol]
        assert len(edges)==sel['expected_count']
        b=bd.fillet(edges,s(f['radius']))
    elif op=='revolve':
        pts=[(s(x),s(y)) for x,y in f['profile']['points']]
        sketch=bd.Polygon(*pts,align=None);fr=f['frame'];x=bd.Vector(vec(fr['x_axis']));y=bd.Vector(vec(fr['y_axis']))
        sketch=frame(sketch,vec(fr['origin']),x,x.cross(y))
        b=bd.revolve(sketch,axis=bd.Axis(vec(f['axis_origin']),vec(f['axis'])),revolution_arc=s(f['angle_deg']))
    elif op=='shell_body':
        h,w,fl=s(f['height']),s(f['wall']),s(f['floor']);pr=f['profile']
        b=prism(pr,h)-prism(pr,h-fl+1,w).translate((0,0,fl))
        r=s(f.get('edge_radius',0))
        if r:
            # Top outer wire only; inner rim remains sharp, matching the input.
            top=b.faces().filter_by(bd.Axis.Z).sort_by(bd.Axis.Z)[-1]
            b=bd.fillet(top.outer_wire().edges(),r)
    elif op=='face_cutout':
        b=bodies[f['input']];o,x,n=face_frame(b,f['face'],vec(f['center']))
        b=b-frame(prism(f['profile'],s(f['depth'])+2e-4),o,x,n)
    elif op=='face_hole_grid':
        b=bodies[f['input']];cx,cy=vec(f['center']);nx,ny=map(int,vec(f['count']));dx,dy=vec(f['spacing']);tools=[]
        for j in range(ny):
            for i in range(nx):
                o,x,n=face_frame(b,f['face'],(cx+(i-(nx-1)/2)*dx,cy+(j-(ny-1)/2)*dy))
                tools.append(frame(prism({'kind':'circle','radius':f['radius']},s(f['depth'])+2e-4),o,x,n))
        b=b.cut(*tools)
    elif op=='boss':
        h,d=s(f['height']),s(f['hole_depth'])
        b=(prism({'kind':'circle','radius':f['outer_radius']},h)-prism({'kind':'circle','radius':f['hole_radius']},d+1).translate((0,0,h-d))).translate(vec(f['origin']))
    elif op=='pattern':b=bd.Compound([bodies[f['input']].translate(vec(v)) for v in f['positions']])
    elif op=='union':b=union(bodies[f['left']],bodies[f['right']])
    elif op=='lid_for':
        pr=f['profile'];w,t,h,c=map(s,[f['wall'],f['thickness'],f['lip_height'],f['clearance']])
        b=union(prism(pr,t).translate((0,0,h)),prism(pr,h,w+c)-prism(pr,h,2*w+c))
    elif op=='surface_patch':
        nets[f['id']]=[[vec(p) for p in row] for row in f['control_points']];b=bd.Face.make_bezier_surface(nets[f['id']])
    elif op=='surface_bridge':
        a=[[bd.Vector(p) for p in row] for row in nets[f['start']]];z=[[bd.Vector(p) for p in row] for row in nets[f['end']]]
        n,q=len(a)-1,len(z)-1;sa,sb=s(f['start_scale']),s(f['end_scale']);v=len(a[0]);out=[[None]*v for _ in range(6)]
        for j in range(v):
            out[0][j]=a[n][j];out[1][j]=out[0][j]+(a[n][j]-a[n-1][j])*(n*sa/5)
            out[2][j]=(a[n][j]-a[n-1][j]*2+a[n-2][j])*(n*(n-1)*sa*sa/20)+out[1][j]*2-out[0][j]
            out[5][j]=z[0][j];out[4][j]=z[0][j]-(z[1][j]-z[0][j])*(q*sb/5)
            out[3][j]=(z[2][j]-z[1][j]*2+z[0][j])*(q*(q-1)*sb*sb/20)+out[4][j]*2-out[5][j]
        nets[f['id']]=[[tuple(p) for p in row] for row in out];b=bd.Face.make_bezier_surface(nets[f['id']])
    elif op=='surface_edit':
        net=copy.deepcopy(nets[f['input']])
        for e in f['edits']:net[e['u']][e['v']]=vec(e['point'])
        nets[f['id']]=net;b=bd.Face.make_bezier_surface(net)
    elif op=='place':
        b=bodies[f['input']]
        for axis,a in zip([bd.Axis.X,bd.Axis.Y,bd.Axis.Z],vec(f['rotation_deg'])):b=b.rotate(axis,a)
        b=b.translate(vec(f['translation']))
    else:raise ValueError('baseline unsupported operation '+op)
    bodies[f['id']]=b
if m.get('assembly'):
    parts=[]
    for i in m['assembly']:
        assert i.get('export',True)
        pos=i.get('poses',{}).get('normal',{})
        b=bodies[i['input']].translate(vec(pos.get('translation',i.get('translation',[0,0,0]))));b.label=i['id'];parts.append(b)
    b=bd.Compound(label='product',children=parts)
else:b=bodies[m['output']]
built=time.perf_counter();valid=b.is_valid
volume=sum(i.volume for i in b.solids());area=b.area
validated=time.perf_counter()
assert bd.export_step(b,sys.argv[1])
finished=time.perf_counter()
print(json.dumps({'ok':valid,'data':{'volume_mm3':volume,'area_mm2':area,'solids':len(b.solids()),'timing_ms':{'import':(loaded-started)*1000,'geometry':(built-loaded)*1000,'validation':(validated-built)*1000,'export':(finished-validated)*1000,'total':(finished-started)*1000},'max_rss_raw':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}}))
