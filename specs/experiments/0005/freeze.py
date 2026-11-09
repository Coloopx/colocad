"""Freeze source-backed G2 inputs/oracles BEFORE native execution. MIT fixtures.
Run only to create a NEW corpus directory; existing manifests are immutable.
Independent reference construction uses OCCT, never Colocad output.
"""
from pathlib import Path
import hashlib,json,math,re,sys,copy,importlib.metadata
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox,BRepPrimAPI_MakeCylinder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut,BRepAlgoAPI_Fuse
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.Geom import Geom_BezierSurface
from OCP.TColgp import TColgp_Array2OfPnt
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.STEPControl import STEPControl_Writer,STEPControl_AsIs,STEPControl_Reader
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.gp import gp_Pnt,gp_Dir,gp_Ax2
repo=Path(__file__).resolve().parents[3];out=repo/'specs/experiments/0005/corpus-v1'
assert not (out/'manifest.json').exists(),'Frozen manifest exists: never overwrite evidence'
out.mkdir(exist_ok=True);cases=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def write(path,value):path.write_text(json.dumps(value,indent=2)+'\n')
def box(origin,size):return BRepPrimAPI_MakeBox(gp_Pnt(*origin),*size).Shape()
def cyl(origin,axis,radius,height):return BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(*origin),gp_Dir(*axis)),radius,height).Shape()
def cut(a,b):
 c=BRepAlgoAPI_Cut(a,b);c.Build();assert c.IsDone();return c.Shape()
def fillet(a,r):
 c=BRepFilletAPI_MakeFillet(a);e=TopExp_Explorer(a,TopAbs_EDGE)
 while e.More():c.Add(r,TopoDS.Edge_s(e.Current()));e.Next()
 c.Build();assert c.IsDone();return c.Shape()
def mdl(features,**kw):return {'version':1,'units':'mm','parameters':{},'features':features,'output':features[-1]['id'],**kw}
def bfeat(id,o,s):return {'id':id,'op':'box','origin':o,'size':s}
def save(id,family,split,model,reference=None,expected_error=None,feature=None,extra=None):
 path=out/(id+'.json');write(path,model);entry={'id':id,'family':family,'split':split,'required':True,'model':path.name,'model_sha256':sha(path),'expect':'reject' if expected_error else 'success','accepted_degradations':[],'units':'mm','provenance':{'kind':'independently_authored_analytic_or_OCCT_fixture','license':'MIT','source':'freeze.py','generator_sha256':sha(Path(__file__))}}
 if expected_error:entry['error']={'codes':expected_error,'feature_id':feature}
 if reference is not None:
  assert BRepCheck_Analyzer(reference).IsValid(),id
  props=GProp_GProps();BRepGProp.VolumeProperties_s(reference,props,Eps=1e-10);area=GProp_GProps();BRepGProp.SurfaceProperties_s(reference,area,Eps=1e-10)
  bounds=Bnd_Box();BRepBndLib.AddOptimal_s(reference,bounds,False,False)
  p=out/(id+'-reference.step');w=STEPControl_Writer();assert int(w.Transfer(reference,STEPControl_AsIs))==1;assert int(w.Write(str(p)))==1
  p.write_text(re.sub(r"'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}'","'2026-11-09T00:00:00'",p.read_text(),count=1))
  entry['oracle']={'file':p.name,'sha256':sha(p),'volume_mm3':props.Mass() if model.get('output_kind')!='surface' else 0,'area_mm2':area.Mass(),'bbox_mm':list(bounds.Get()),'solids':0 if model.get('output_kind')=='surface' else 1,'comparison':'bezier_samples' if model.get('output_kind')=='surface' else 'symmetric_difference_volume'}
 if extra:entry.update(extra)
 cases.append(entry)
# Crossing three bores plus a pocket: all intersections and wall material matter.
for scale,split in [(1,'development'),(.73,'regression'),(1.31,'held_out')]:
 S=lambda v:[x*scale for x in v];body=box(S([-30,-20,0]),S([60,40,24]));fs=[bfeat('stock',S([-30,-20,0]),S([60,40,24]))]
 definitions=[('z',S([0,0,-1]),[1,0,0],[0,1,0],[0,0,1],4*scale,26*scale),('x',S([-31,0,12]),[0,1,0],[0,0,1],[1,0,0],3*scale,62*scale),('y',S([12,-21,12]),[0,0,1],[1,0,0],[0,1,0],2.5*scale,42*scale)]
 for id,o,x,y,axis,r,h in definitions:
  fs.append({'id':id,'op':'holes','input':fs[-1]['id'],'frame':{'origin':o,'x_axis':x,'y_axis':y},'positions':[[0,0]],'radius':r,'depth':h});body=cut(body,cyl(o,axis,r,h))
 fs.extend([bfeat('pocket',S([-18,-12,18]),S([36,24,8])),{'id':'result','op':'cut','left':'y','right':'pocket'}]);body=cut(body,box(S([-18,-12,18]),S([36,24,8])))
 save('boolean-'+split,'boolean',split,mdl(fs),body)
save('boolean-empty-cut','boolean','regression',mdl([bfeat('stock',[0,0,0],[20,20,20]),bfeat('tool',[50,0,0],[5,5,5]),{'id':'result','op':'cut','left':'stock','right':'tool'}]),expected_error=['NO_MATERIAL_REMOVED'],feature='result')
# All twelve convex edges blend together, followed by four functional bores.
for size,r,split in [([50,34,16],1.2,'regression'),([63,39,19],2.1,'held_out')]:
 fs=[bfeat('stock',[0,0,0],size),{'id':'round','op':'fillet','input':'stock','radius':r,'select':{'kind':'all'}},{'id':'result','op':'holes','input':'round','frame':{'origin':[0,0,-1],'x_axis':[1,0,0],'y_axis':[0,1,0]},'positions':[[8,8],[size[0]-8,8],[8,size[1]-8],[size[0]-8,size[1]-8]],'radius':2.2,'depth':size[2]+2}]
 body=fillet(box([0,0,0],size),r)
 for x,y in fs[-1]['positions']:body=cut(body,cyl([x,y,-1],[0,0,1],2.2,size[2]+2))
 save('blend-'+split,'blend',split,mdl(fs),body)
save('blend-impossible','blend','regression',mdl([bfeat('stock',[0,0,0],[10,10,10]),{'id':'round','op':'fillet','input':'stock','radius':50,'select':{'kind':'all'}}]),expected_error=['KERNEL_ERROR','FILLET_FAILED'],feature='round')
# High aspect-ratio closed and open thin-wall housings, exact cavity oracles.
for size,t,opened,split in [([80,50,24],.25,False,'regression'),([127,43,31],.4,True,'held_out')]:
 origin=[t]*3;inner=[v-2*t for v in size];inner[2]=size[2] if opened else inner[2]
 body=cut(box([0,0,0],size),box(origin,inner))
 fs=[bfeat('stock',[0,0,0],size),{'id':'result','op':'shell','input':'stock','thickness':t}]
 if opened:fs[-1]['open_faces']=[{'kind':'on_plane','axis':'z','value':size[2],'expected_count':1}]
 save('thin-'+split,'thin_wall',split,mdl(fs),body,extra={'minimum_wall_mm':t})
fs=[{'id':'stock','op':'extrude','profile':{'kind':'polygon','points':[[0,0],[30,0],[30,10],[10,10],[10,30],[0,30]]},'height':20},{'id':'result','op':'hollow','input':'stock','wall':2,'cavity_strategy':'scaled'}]
save('thin-protruding-cavity','thin_wall','regression',mdl(fs),expected_error=['HOLLOW_OUTSIDE'],feature='result')
# Actual nonplanar Bezier sheets with an independent surface, analytic sample points.
for rise,split in [(3,'regression'),(7.1,'held_out')]:
 net=[[[0,0,0],[0,8,1],[0,16,0]],[[12,0,1],[12,8,rise],[12,16,1]],[[24,0,0],[24,8,1],[24,16,0]]];poles=TColgp_Array2OfPnt(1,3,1,3)
 for i,row in enumerate(net):
  for j,p in enumerate(row):poles.SetValue(i+1,j+1,gp_Pnt(*p))
 surface=Geom_BezierSurface(poles);body=BRepBuilderAPI_MakeFace(surface,1e-9).Face();samples=[]
 for i in range(9):
  for j in range(9):
   p=surface.Value(i/8,j/8);samples.append([p.X(),p.Y(),p.Z()])
 save('surface-'+split,'surface',split,mdl([{'id':'skin','op':'surface_patch','control_points':net}],output_kind='surface'),body,extra={'surface_samples_mm':samples})
save('surface-ragged','surface','regression',mdl([{'id':'skin','op':'surface_patch','control_points':[[[0,0,0],[0,10,0]],[[10,0,0]]]}],output_kind='surface'),expected_error=['INVALID_SURFACE'],feature=None)
network={'id':'skin','op':'surface_network','u_stations':[0,.5,1],'v_stations':[0,.5,1],'u_curves':[[[0,0,0],[5,0,0],[10,0,0]],[[0,5,0],[5,5,4],[10,5,0]],[[0,10,0],[5,10,0],[10,10,0]]],'v_curves':[[[0,0,0],[0,5,0],[0,10,0]],[[5,0,0],[5,5,8],[5,10,0]],[[10,0,0],[10,5,0],[10,10,0]]]}
save('surface-crossing-mismatch','surface','regression',mdl([network],output_kind='surface'),expected_error=['NETWORK_MISMATCH'],feature=None)
# Foreign geometry is independent of native feature construction, including NURBS equivalents.
foreign=repo/'specs/experiments/0004/fixtures/engineering';fm=json.loads((foreign/'manifest.json').read_text())
for i,id in enumerate(['motor-housing','motor-rotated','split-axial-bore','split-angular-bore','spline-bore','conical-tip','intersecting-bores','kk60-accessory-bracket']):
 row=next(c for c in fm['cases'] if c['id']==id);path=foreign/row['file'];assert sha(path)==row['sha256'];step=path.read_text();reader=STEPControl_Reader();assert int(reader.ReadFile(str(path)))==1;reader.TransferRoots()
 model=mdl([{'id':'result','op':'import_step','asset':'source','body_index':0}],assets={'source':{'step':step,'sha256':sha(path)}})
 save('foreign-'+id,'foreign_step','held_out' if i>=6 else 'regression',model,reader.OneShape(),extra={'upstream_source':str(path.relative_to(repo)),'upstream_sha256':sha(path)})
bad=copy.deepcopy(model);bad['assets']['source']['sha256']='0'*64
save('foreign-digest-mismatch','foreign_step','regression',bad,expected_error=['ASSET_INTEGRITY'],feature=None)
# Sourced whole-product model with 82 occurrences; reused correctness target, not claimed held out.
model=json.loads((repo/'out/kk60-interfaces-final/model.json').read_text())
save('product-kk60-accessory','whole_product','regression',model,extra={'oracle':{'verifier':'../../../examples/engineering-interfaces/verify-kk60.py','expected_solids':82,'expected_pairs':3,'expected_samples':9},'provenance':{'kind':'catalog_backed_reconstruction_with_independent_accessory','sources':'examples/hiwin-kk60/sources.json','source_sha256':sha(repo/'examples/hiwin-kk60/sources.json'),'scope':'catalog reconstruction; not OEM manufacturing geometry'},'tolerance_override':{'linear_mm':.01,'angular_rad':1e-4},'bracket_source':next(c for c in fm['cases'] if c['id']=='kk60-accessory-bracket')})
manifest={'schema':'colocad.g2-corpus/1','frozen_before_native_execution':True,'created':'2026-11-09','policy':'All required cases must pass. Never delete failures or loosen tolerances; preserve failed iterations. Held-out cases execute only after regression passes.','source_commit':__import__('subprocess').check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'verifier':{'name':'independent OCCT exact STEP readback','ocp_version':importlib.metadata.version('cadquery-ocp-novtk')},'tolerance':{'linear_mm':1e-5,'angular_rad':1e-7,'relative_volume':1e-6,'absolute_volume_mm3':1e-6,'relative_area':1e-6,'surface_sample_mm':1e-5},'representation':'exact B-rep solids or explicit B-spline sheets, no required degradation','execution':{'per_case_wall_seconds':180,'native_stdout_limit_mb':16,'queryable':True,'preview':False},'cases':cases}
write(out/'manifest.json',manifest);(out/'manifest.sha256').write_text(sha(out/'manifest.json')+'\n');print(json.dumps({'cases':len(cases),'manifest_sha256':sha(out/'manifest.json')}))
