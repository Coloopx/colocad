"""Untimed OCCT readbacks: every successful fixture, every counted hand delivery."""
import hashlib,json,sys
from pathlib import Path
import build123d as b
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
out=Path(sys.argv[1]);mode=sys.argv[2];checks=[];references={}
def vol(s):
    p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,1e-9,True,False);return p.Mass()
def sizes(obj):
    box=obj.bounding_box();return [*box.min,*box.max],vol(obj.wrapped)
def diff(a,b):
    c=BRepAlgoAPI_Cut(a,b);c.SetFuzzyValue(1e-6);c.Build();assert c.IsDone() and BRepCheck_Analyzer(c.Shape()).IsValid()
    return abs(vol(c.Shape()))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def verify(id,case,path,doc,rep,guide=None,cable=None):
    obj=b.import_step(path);parts=list(obj.solids());assert len(parts)==len(doc.get('assembly',[0])) and all(s.is_valid for s in parts)
    vals=[sizes(s) for s in parts];key=case
    metrics={'id':id,'sha256':sha(path),'solids':len(parts),'volume':sum(v for _,v in vals)}
    if key not in references:references[key]=path,vals
    elif references[key][0]!=path:
        ref,rv=references[key];pool=list(enumerate(rv));pairs=[]
        for ar,av in vals:
            pos=min(range(len(pool)),key=lambda j:max(abs(a-b) for a,b in zip(ar,pool[j][1][0]))+abs(av-pool[j][1][1])/max(av,1))
            idx,(br,bv)=pool.pop(pos);err=max(abs(a-b) for a,b in zip(ar,br));assert err<1e-4 and abs(av-bv)/max(av,1)<1e-6,(id,av,bv,err)
            pairs.append(idx)
        if rep==0 and mode=='fixtures':
            refparts=list(b.import_step(ref).solids());delta=[]
            for part,idx in zip(parts,pairs):
                a,z=part.wrapped,refparts[idx].wrapped
                d=diff(a,z)+diff(z,a);assert d/max(vol(a),1)<1e-6,(id,idx,d);delta.append(d)
            metrics['max_two_way_difference_mm3']=max(delta)
    if case.startswith('sensor-rail'):
        target=doc['parameters'];expected=(target['step'],target['copies'])
        for y in (3,20,22,41):
            # Use centers of the separated first and last slots; two middle slots overlap.
            if y in (20,22):continue
            hole=any(abs(y-(3+j*expected[0]))<2.5 for j in range(expected[1]))
            assert parts[0].is_inside((0,y,4))!=hole
        metrics['slot_count_parameter']=expected[1]
    if case.startswith('manifold'):
        p=doc['parameters'];rad=p['pocketRadius'];n=p['instances']
        for i in range(4):
            x=14+i*p['pitch']
            assert parts[0].is_inside((x,12,4))==(i>=n)
            assert parts[0].is_inside((x+rad+.2,12,4))
            if i<n:assert not parts[0].is_inside((x+rad-.2,12,4))
        metrics['checked_bore_diameter']=2*rad;metrics['checked_bore_count']=n
    if guide is not None:
        # Locate by geometry, independent of STEP child order/names or exporter hierarchy.
        inst={i['id']:i['translation'] for i in doc['assembly']}
        def centered(box,x,y):
            return abs((box.min.X+box.max.X)/2-x)<1e-5 and abs((box.min.Y+box.max.Y)/2-y)<1e-5
        x,y,z=inst['Index_Proximal'];section_z=z+23
        links=[s for s in parts if centered(s.bounding_box(),x,y) and s.bounding_box().min.Z<section_z<s.bounding_box().max.Z]
        assert len(links)==1,(id,'index proximal',len(links))
        sec=b.section(links[0],section_by=b.Plane.XY.offset(section_z))
        holes=[2*e.radius for e in sec.edges() if e.geom_type==b.GeomType.CIRCLE]
        assert len(holes)==1 and abs(holes[0]-guide)<1e-5,(id,guide,holes)
        x,y,z=inst['Index_Tendon'];cords=[s.bounding_box() for s in parts if centered(s.bounding_box(),x,y) and abs(s.bounding_box().min.Z-z)<1e-5 and s.bounding_box().size.Z>50]
        assert len(cords)==1 and abs(cords[0].size.X/2-cable)<1e-5,(id,'index cord')
        metrics.update(checked_guide_diameter=guide,checked_cable_radius=cable)
    checks.append(metrics)
if mode=='fixtures':
    for r in sorted(json.loads((out/'fixtures/runs.json').read_text()),key=lambda r:(r['rep'],r['case'],r['backend'])):
        if r['status']=='ok':
            case=r['case'];path=out/'fixtures'/r['id']/r['result']['step_path']
            verify(r['id'],case,path,json.loads((out/'inputs'/(case+'.json')).read_text()),r['rep'])
else:
    doc=json.loads((out/'inputs/hand.json').read_text())
    for backend in ('cadquery','colocad','build123d'):
        d=out/'windows'/backend
        if not (d/'environment.json').exists():continue
        for line in (d/'events.jsonl').read_text().splitlines():
            e=json.loads(line);assert e['in_window']==(e['end_s']<=100)
            if e['in_window']:
                i=e['revision'];path=d/f'{i:03d}.step';assert sha(path)==e['step_sha256']
                verify(backend+'/'+str(i),'hand/'+str(i),path,doc,i,e['guide_diameter_mm'],e['cable_radius_mm'])
(out/(mode+'-verified.json')).write_text(json.dumps(checks,indent=2)+'\n');print(mode,len(checks),'readbacks passed')
