"""Read both independent STEP exports using one OCCT evaluator and mesh pipeline.
Run AFTER benchmarking to avoid loading the measurement host during timing trials.
"""
import base64,hashlib,json,math,time
from pathlib import Path
import build123d as bd
import numpy as np
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer
from cases import ROOT,cases

def props(shape,surface=False):
    p=GProp_GProps()
    if surface:BRepGProp.SurfaceProperties_s(shape,p,1e-9)
    else:BRepGProp.VolumeProperties_s(shape,p,1e-9,True,False)
    return p.Mass()
def encode(a,dtype):return base64.b64encode(np.asarray(a,dtype=dtype).tobytes()).decode()
def render_parts(shape,name):
    parts=list(shape.solids()) if len(shape.solids()) else list(shape.faces());meshes=[]
    for part in parts:
        dim=list(part.bounding_box().size);offset=[0,0,0];label='part';color=[0.66,0.75,0.78]
        if name=='product':
            if abs(dim[2]-45)<0.01:label='housing';color=[0.72,0.78,0.8]
            elif abs(dim[2]-5)<0.01:label='lid';offset=[0,0,23];color=[0.79,0.84,0.85]
            elif abs(dim[2]-1.6)<0.01:label='PCB';offset=[0,0,15];color=[0.19,0.48,0.39]
            elif abs(dim[2]-12)<0.01:label='display';offset=[0,-12,0];color=[0.15,0.23,0.29]
            else:raise ValueError('unexpected product part dimensions '+str(dim))
        vertices,triangles=part.tessellate(0.12,0.12)
        v=np.array([list(p) for p in vertices],dtype=np.float64);t=np.array(triangles,dtype=np.int64)
        normals=np.zeros(v.shape);cross=np.cross(v[t[:,1]]-v[t[:,0]],v[t[:,2]]-v[t[:,0]])
        for i in range(3):np.add.at(normals,t[:,i],cross)
        normals/=np.maximum(np.linalg.norm(normals,axis=1,keepdims=True),1e-15)
        meshes.append({'label':label,'color':color,'offset':offset,'vertices':encode(v,'<f4'),'normals':encode(normals,'<f4'),'indices':encode(t,'<u4'),'vertex_count':len(v),'triangles':len(t),'bounds':[v.min(0).tolist(),v.max(0).tolist()]})
    return meshes
results={};assets={}
for name,(title,category,description,model) in cases().items():
    shapes={};steps={};row={'title':title,'category':category,'description':description,'model':model,'backends':{}}
    for backend in ['rust','build123d']:
        f=ROOT/'data'/name/backend/('revision/model.step' if backend=='rust' else 'model.step')
        t=time.perf_counter();shape=bd.import_step(f);read_ms=(time.perf_counter()-t)*1000
        assert shape.is_valid,(name,backend)
        shapes[backend]=shape;steps[backend]=f.read_text()
        bb=shape.bounding_box();solids=len(shape.solids())
        assert solids==(0 if name=='surface' else 4 if name=='product' else 1)
        row['backends'][backend]={'valid':shape.is_valid,'solids':solids,'faces':len(shape.faces()),'volume_mm3':props(shape.wrapped) if solids else None,'area_mm2':props(shape.wrapped,True),'bbox_mm':[list(bb.min),list(bb.max)],'step_bytes':f.stat().st_size,'step_sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'readback_ms_unbenchmarked':read_ms}
    a,b=(row['backends'][k] for k in ['rust','build123d'])
    row['bbox_max_delta_mm']=max(abs(x-y) for xs,ys in zip(a['bbox_mm'],b['bbox_mm']) for x,y in zip(xs,ys))
    row['area_relative_delta']=abs(a['area_mm2']-b['area_mm2'])/b['area_mm2']
    if a['volume_mm3'] is not None:
        row['volume_relative_delta']=abs(a['volume_mm3']-b['volume_mm3'])/b['volume_mm3']
        diff=[]
        for first,second in [('rust','build123d'),('build123d','rust')]:
            cut=BRepAlgoAPI_Cut(shapes[first].wrapped,shapes[second].wrapped);cut.SetFuzzyValue(1e-6);cut.Build()
            assert cut.IsDone() and BRepCheck_Analyzer(cut.Shape()).IsValid()
            diff.append(abs(props(cut.Shape())))
        row['symmetric_difference_mm3']=sum(diff)
        row['symmetric_difference_relative']=sum(diff)/max(a['volume_mm3'],b['volume_mm3'])
        row['geometry_pass']=row['volume_relative_delta']<1e-6 and row['symmetric_difference_relative']<1e-6 and row['bbox_max_delta_mm']<1e-4
    else:
        # Sample both analytic STEP faces on a 15x15 normalized parameter grid.
        # This is an explicit sampling check, not a global Hausdorff certificate.
        f1,f2=shapes['rust'].faces()[0],shapes['build123d'].faces()[0]
        errors=[]
        for u in np.linspace(0,1,15):
            for v in np.linspace(0,1,15):
                errors.append((f1.position_at(float(u),float(v))-f2.position_at(float(u),float(v))).length)
        row['surface_sample_max_delta_mm']=max(errors);row['surface_sample_count']=len(errors)
        row['geometry_pass']=row['surface_sample_max_delta_mm']<1e-4 and row['area_relative_delta']<1e-6
    meshes={k:render_parts(v,name) for k,v in shapes.items()}
    for k in row['backends']:row['backends'][k]['display_triangles']=sum(m['triangles'] for m in meshes[k])
    results[name]=row;assets[name]={'meshes':meshes,'steps':steps}
    print(name,'pass' if row['geometry_pass'] else 'DIFFERENCE',row.get('volume_relative_delta',row.get('surface_sample_max_delta_mm')),flush=True)
(ROOT/'data/geometry.json').write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n')
(ROOT/'data/viewer-assets.json').write_text(json.dumps(assets,separators=(',',':'))+'\n')
