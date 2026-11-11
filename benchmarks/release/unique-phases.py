"""Compare actual phases for the median-wall-time run of the same 64-part input."""
import argparse,json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=argparse.ArgumentParser();p.add_argument('before',type=Path);p.add_argument('after',type=Path);p.add_argument('dest',type=Path);a=p.parse_args()
def sample(root,backend):
    candidates=[]
    for f in (root/'main').glob(backend+'-unique-64-r*/measurement.json'):
        r=json.loads(f.read_text());assert r['status']=='ok'
        candidates.append((r['wall_s'],r,f.parent))
    assert len(candidates)==3
    wall,r,d=sorted(candidates,key=lambda item:item[0])[1]
    if backend=='colocad':
        t=r['result']['data']['timing_ms']
        g,v,x=[t[field]/1000 for field in ['geometry','validation','export']]
    else:
        ends={e['phase']:e['elapsed_s'] for e in map(json.loads,(d/'events.jsonl').read_text().splitlines())}
        g,v,x=ends['geometry']-ends['imported'],ends['validated']-ends['geometry'],ends['exported']-ends['validated']
    return dict(run=d.name,wall_s=wall,geometry_s=g,validation_s=v,export_s=x,other_s=wall-g-v-x)
b=json.loads((a.before/'campaign.json').read_text())['engine_commit'][:7];n=json.loads((a.after/'campaign.json').read_text())['engine_commit'][:7]
rows={f'Colocad {b}':sample(a.before,'colocad'),f'Colocad {n}':sample(a.after,'colocad'),'CadQuery 2.8.0':sample(a.after,'cadquery'),'build123d 0.13.0':sample(a.after,'build123d')}
(a.dest/'unique-phases.json').write_text(json.dumps(rows,indent=2)+'\n')
plt.rcParams.update({'font.family':'DejaVu Sans','svg.fonttype':'none','svg.hashsalt':'unique-phases'})
fig,ax=plt.subplots(figsize=(10.5,3.7));fig.patch.set_facecolor('#f6f8f5');fig.subplots_adjust(left=.20,right=.93,top=.72,bottom=.23)
fig.text(.045,.91,'Where the time goes for 64 distinct plates',fontsize=19,weight='bold',color='#192f36')
fig.text(.045,.82,'One median-wall-time run per backend and revision · 1 vCPU / 2 GiB · seconds',fontsize=11,color='#425a60')
left=[0.]*len(rows)
for field,label,color in [('other_s','Startup and other','#acc5c3'),('geometry_s','Geometry','#257d78'),('validation_s','Placement and validation','#a86125'),('export_s','STEP export','#605882')]:
    widths=[r[field] for r in rows.values()]
    ax.barh(list(rows),widths,left=left,label=label,color=color,height=.55)
    left=[s+w for s,w in zip(left,widths)]
for i,total in enumerate(left):ax.text(total+.25,i,f'{total:.2f}',va='center',fontsize=10)
ax.invert_yaxis();ax.set_xlim(0,max(left)*1.12);ax.grid(axis='x',alpha=.2);ax.set_axisbelow(True);ax.spines[['top','right','left']].set_visible(False)
fig.legend(*ax.get_legend_handles_labels(),loc='lower center',ncol=4,frameon=False)
fig.savefig(a.dest/'unique-phases.svg',metadata={'Date':None});plt.close(fig)

svg=a.dest/"unique-phases.svg"
svg.write_text("\n".join(line.rstrip() for line in svg.read_text().splitlines())+"\n")
