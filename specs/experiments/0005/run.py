"""Frozen G2 correctness runner. Python/OCCT are independent validation, not CAD runtime.
Persist every attempt; do not replace an existing run directory.
"""
from pathlib import Path
import os,sys,json,hashlib,time,subprocess,math,traceback,collections
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.gp import gp_Pnt
root=Path(__file__).resolve().parent;repo=root.parents[2];corpus=root/'corpus-v1';manifest=json.loads((corpus/'manifest.json').read_text());sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(corpus/'manifest.json')==(corpus/'manifest.sha256').read_text().strip()
binary=Path(os.environ['AGENT_CAD_BINARY']).resolve();out=Path(sys.argv[1]).resolve();out.mkdir(parents=True,exist_ok=False);split=sys.argv[2] if len(sys.argv)>2 else 'regression';tol=manifest['tolerance'];rows=[]
assert split in ['regression','held_out','all']
reuse=Path(sys.argv[3]).resolve() if len(sys.argv)>3 else None
prior=json.loads((reuse/'results.json').read_text()) if reuse else None
if prior:assert prior['manifest_sha256']==sha(corpus/'manifest.json')
def read(path):
 r=STEPControl_Reader();assert int(r.ReadFile(str(path)))==1;r.TransferRoots();return r.OneShape()
def entities(s,kind):
 r=[];e=TopExp_Explorer(s,kind)
 while e.More():r.append(e.Current());e.Next()
 return r
def volume(s):
 p=GProp_GProps();BRepGProp.VolumeProperties_s(s,p,Eps=1e-10);return p.Mass()
def area(s):
 p=GProp_GProps();BRepGProp.SurfaceProperties_s(s,p,Eps=1e-10);return p.Mass()
def bounds(s):
 b=Bnd_Box();BRepBndLib.AddOptimal_s(s,b,False,False);return list(b.Get())
def difference_volume(a,b):
 op=BRepAlgoAPI_Cut(a,b);op.Build();assert op.IsDone(),'independent Boolean failed';return sum(abs(volume(s)) for s in entities(op.Shape(),TopAbs_SOLID))
def near(got,want,limit,name):assert abs(got-want)<=limit,(name,got,want,limit)
def validate(case,result,directory):
 assert result['ok'],result.get('error')
 d=result['data'];assert not any(f.get('degradation') is not None for f in d['features']),'required feature degraded'
 step=directory/'model.step';assert sha(step)==d['artifact_sha256']['model.step'];shape=read(step);assert BRepCheck_Analyzer(shape).IsValid(),'OCCT invalid B-rep'
 if case['family']=='whole_product':
  run={'source':case['bracket_source'],'built':{'ok':True,'data':{**d,'artifact_directory':str(directory)}}};(directory/'run.json').write_text(json.dumps(run));cmd=[sys.executable,str(repo/'examples/engineering-interfaces/verify-kk60.py'),str(directory/'run.json')];p=subprocess.run(cmd,capture_output=True,text=True,timeout=120);(directory/'independent.log').write_text(p.stdout+p.stderr);assert p.returncode==0,p.stderr
  return json.loads((directory/'occt-readback.json').read_text())
 oracle=case['oracle'];assert sha(corpus/oracle['file'])==oracle['sha256'];reference=read(corpus/oracle['file']);ss=entities(shape,TopAbs_SOLID);assert len(ss)==oracle['solids'],('solid_count',len(ss),oracle['solids']);v=sum(volume(s) for s in ss);a=area(shape);bb=bounds(shape)
 vtol=max(tol['absolute_volume_mm3'],abs(oracle['volume_mm3'])*tol['relative_volume']);near(v,oracle['volume_mm3'],vtol,'volume');near(a,oracle['area_mm2'],max(1e-6,oracle['area_mm2']*tol['relative_area']),'area')
 # Native report volume is independently checked too; sheets explicitly have no material volume.
 near(d['geometry']['volume_mm3'],oracle['volume_mm3'],vtol,'native_volume')
 for got,want in zip(bb,oracle['bbox_mm']):near(got,want,tol['linear_mm'],'bbox')
 evidence={'valid':True,'solids':len(ss),'faces':len(entities(shape,TopAbs_FACE)),'volume_mm3':v,'area_mm2':a,'bbox_mm':bb,'step_sha256':sha(step)}
 if oracle['comparison']=='symmetric_difference_volume':
  ab=difference_volume(shape,reference);ba=difference_volume(reference,shape);assert ab+ba<=vtol,('material_symmetric_difference',ab,ba,vtol);evidence['symmetric_difference_mm3']=ab+ba
 else:
  distances=[]
  for point in case['surface_samples_mm']:
   q=BRepBuilderAPI_MakeVertex(gp_Pnt(*point)).Vertex();m=BRepExtrema_DistShapeShape(q,shape);m.Perform();assert m.IsDone();distances.append(m.Value())
  assert max(distances)<=tol['surface_sample_mm'];evidence['surface_samples']=len(distances);evidence['max_surface_error_mm']=max(distances)
 return evidence
selected=[c for c in manifest['cases'] if split=='all' or (c['split']=='held_out')==(split=='held_out')]
record={'schema':'colocad.g2-run/1','manifest_sha256':sha(corpus/'manifest.json'),'native_sha256':sha(binary),'split':split,'rows':rows,'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()}
if prior:
 record.update(native_sha256=prior['native_sha256'],source_commit=prior['source_commit'],revalidated_artifacts_from=str(reuse))
for case in selected:
 path=corpus/case['model'];assert sha(path)==case['model_sha256'];directory=(reuse or out)/case['id'];started=time.monotonic();row={'id':case['id'],'family':case['family'],'expect':case['expect'],'passed':False};rows.append(row)
 try:
  if reuse:
   raw=(reuse/(case['id']+'-stdout.json')).read_bytes();stderr=(reuse/(case['id']+'-stderr.log')).read_bytes();process_exit=next(r['process_exit'] for r in prior['rows'] if r['id']==case['id'])
  else:
   p=subprocess.run([str(binary),'build','--out',str(directory),'--queryable'],input=path.read_bytes(),capture_output=True,timeout=manifest['execution']['per_case_wall_seconds']);raw=p.stdout;stderr=p.stderr;process_exit=p.returncode
  (out/(case['id']+'-stdout.json')).write_bytes(raw);(out/(case['id']+'-stderr.log')).write_bytes(stderr)
  assert len(raw)<16*1024*1024,'stdout budget';result=json.loads(raw);row['process_exit']=process_exit
  if case['expect']=='reject':
   assert result['ok'] is False,result;error=result['error'];assert error['code'] in case['error']['codes'],error
   if case['error']['feature_id']:assert error['feature_id']==case['error']['feature_id'],error
   assert not directory.exists(),'failed build committed output';row['rejection']=error
  else:row['verification']=validate(case,result,directory)
  row['passed']=True
 except Exception as error:
  row['failure']=str(error);row['traceback']=traceback.format_exc();row['outcome']='timeout' if isinstance(error,subprocess.TimeoutExpired) else 'verification_or_build_failure'
 row['wall_seconds']=time.monotonic()-started
 print(json.dumps({k:v for k,v in row.items() if k not in ['verification','traceback']}),flush=True)
 record['families']={f:{'required':sum(r['family']==f for r in rows),'passed':sum(r['family']==f and r['passed'] for r in rows),'positive':sum(r['family']==f and r['expect']=='success' for r in rows),'expected_rejections':sum(r['family']==f and r['expect']=='reject' for r in rows)}for f in sorted({r['family'] for r in rows})}
 record['passed']=all(r['passed'] for r in rows) and len(rows)==len(selected);(out/'results.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'passed':record['passed'],'cases':len(rows),'families':record['families']}));sys.exit(0 if record['passed'] else 1)
