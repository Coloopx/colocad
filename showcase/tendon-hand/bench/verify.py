"""After timing: read every counted STEP, compare every placed solid against the other backend."""
import argparse,hashlib,json,os,statistics,subprocess,sys,tempfile
from pathlib import Path
import build123d as b
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
from OCP.BRepCheck import BRepCheck_Analyzer

def vol(shape):
    p=GProp_GProps();BRepGProp.VolumeProperties_s(shape,p,1e-9,True,False);return p.Mass()
def describe(path):
    s=b.import_step(path);parts=s.children
    assert len(parts)==43 and len(s.solids())==43 and s.is_valid
    def metrics(c):
        box=c.bounding_box();return [*box.min,*box.max],vol(c.wrapped)
    return parts,[metrics(c) for c in parts]
def difference(a,b):
    algo=BRepAlgoAPI_Cut(a,b);algo.SetFuzzyValue(1e-6);algo.Build()
    assert algo.IsDone() and BRepCheck_Analyzer(algo.Shape()).IsValid()
    return abs(vol(algo.Shape()))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('run',type=Path);p.add_argument('model',type=Path);p.add_argument('project',type=Path);p.add_argument('out',type=Path);p.add_argument('--native-arm',choices=('colocad','optimized'),default='colocad');p.add_argument('--supplemental-kernel',type=Path);a=p.parse_args()
    tool=Path(__file__).resolve().parents[3]/'skills/colocad/scripts/build.mjs'
    arms=[a.native_arm,'build123d'];doc=json.loads(a.model.read_text());events={};verified=[];differences=[]
    for arm in arms:
        events[arm]=[json.loads(r) for r in (a.run/arm/'events.jsonl').read_text().splitlines()]
        assert events[arm] and all(e['start_s']<100 for e in events[arm])
        for e in events[arm]:
            assert e['in_window']==(e['end_s']<=100)
            assert hashlib.sha256((a.run/arm/f"{e['revision']:03d}.step").read_bytes()).hexdigest()==e['step_sha256']
    n=max(sum(e['in_window'] for e in events[k]) for k in arms)
    with tempfile.TemporaryDirectory() as tmp:
        t=Path(tmp)
        for i in range(n):
            have={k:(events[k][i] if i<len(events[k]) else None) for k in arms}
            obs=next(x for x in have.values() if x)
            vals=doc['parameters'];vals.update(cableRadius=obs['cable_radius_mm'],guideRadius=obs['guide_diameter_mm']/2)
            src=t/'input.json';src.write_text(json.dumps(doc,separators=(',',':')))
            assert all(z is None or hashlib.sha256(src.read_bytes()).hexdigest()==z['model_sha256'] for z in have.values())
            srcs={}
            for k,e in have.items():
                if e:srcs[k]=a.run/k/f'{i:03d}.step'
                elif k==a.native_arm:
                    env = os.environ.copy()
                    if a.native_arm=='optimized':
                        if not a.supplemental_kernel: raise RuntimeError('Missing supplemental optimized binary for untimed counterpart')
                        env['AGENT_CAD_BINARY']=str(a.supplemental_kernel.resolve())
                    r=json.loads(subprocess.check_output(['node',tool,src,t/f'cad-{i}'],cwd=a.project.resolve(),env=env,text=True));srcs[k]=Path(r['directory'])/'model.step'
                else:
                    srcs[k]=t/f'py-{i}.step';subprocess.run([sys.executable,Path(__file__).with_name('baseline.py'),src,srcs[k]],check=True,stdout=subprocess.DEVNULL)
            part_a,ma=describe(srcs[arms[0]]);part_b,mb=describe(srcs[arms[1]])
            # Both independent writers emit occurrences in the input assembly order. Pair by
            # position/extent/volume too; check all positions, not just the top-level box.
            bbox=max(max(abs(x-y) for x,y in zip(pa[0],pb[0])) for pa,pb in zip(ma,mb))
            rel=max(abs(pa[1]-pb[1])/max(pb[1],1) for pa,pb in zip(ma,mb))
            assert bbox<1e-4 and rel<1e-6,(i,bbox,rel)
            if i==0:
                seen=set()
                for ca,cb,ins in zip(part_a,part_b,doc['assembly']):
                    if ins['input'] in seen:continue
                    seen.add(ins['input']);d=[difference(ca.wrapped,cb.wrapped),difference(cb.wrapped,ca.wrapped)]
                    assert sum(d)/max(vol(ca.wrapped),1)<1e-6,(ins,d)
                    differences.append({'definition':ins['input'],'differences_mm3':d})
            bore_readback={}
            lengths={}
            for arm_key,parts in zip(arms,(part_a,part_b)):
                by_id={ins['id']:part for ins,part in zip(doc['assembly'],parts,strict=True)}
                section=b.section(by_id['Index_Proximal'],section_by=b.Plane.XY.offset(117))
                circles=[edge for edge in section.edges() if edge.geom_type==b.GeomType.CIRCLE]
                assert len(circles)==1
                bore_readback[arm_key]=2*circles[0].radius
                lengths[arm_key]=by_id['Index_Tendon'].bounding_box().size.X/2
            assert all(abs(value-obs['guide_diameter_mm'])<1e-5 for value in bore_readback.values())
            assert all(abs(value-obs['cable_radius_mm'])<1e-5 for value in lengths.values())
            verified.append({'revision':i,'step_guide_diameter_mm':bore_readback,'step_cable_radius_mm':lengths,'solids':43,'bbox_max_mm':bbox,'max_part_relative_volume_delta':rel,
                             'outside_window_reference':[k for k,e in have.items() if not e or not e['in_window']]})
            print('verified',i,flush=True)
    common=min(sum(e['in_window'] for e in events[k]) for k in arms)
    data={'window_s':100,'common_revisions':common,'summary':{},'events':events,'environments':{k:json.loads((a.run/k/'environment.json').read_text()) for k in arms},
          'base_image':(a.run/'base-image.txt').read_text().strip(),'verified_pairs':verified,'first_pair_boolean_differences':differences}
    for k in arms:
        within=[e for e in events[k] if e['in_window']]
        data['summary'][k]={'count':len(within),'first_s':within[0]['job_s'],'matched_median_s':statistics.median(e['job_s'] for e in within[:common]),'last_completed_at_s':within[-1]['end_s']}
    traces={}
    for arm in arms:
        memory_file=a.run/arm/'memory.json'
        if memory_file.exists():
            samples=json.loads(memory_file.read_text())
            assert samples[0][0]==0 and samples[-1][0]>99 and all(0<=t<=100 and used>0 for t,used in samples)
            assert all(left[0]<right[0] for left,right in zip(samples,samples[1:]))
            environment=data['environments'][arm]
            assert len(samples)==environment['sample_count']
            assert max(used for t,used in samples)==environment['sampled_window_peak_bytes']
            assert int(dict(line.split() for line in environment['oom_events'].splitlines())['oom_kill'])==0
            data['summary'][arm]['sampled_peak_mib']=environment['sampled_window_peak_bytes']/2**20
            traces[arm]=samples
    if traces:
        assert len(traces)==len(arms)
        data['memory_samples_s_bytes']=traces
    a.out.write_text(json.dumps(data,indent=2)+'\n')
