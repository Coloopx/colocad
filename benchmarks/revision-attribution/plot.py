"""Plot original cgroup components and the single-layout intervention."""
import json
from pathlib import Path
from statistics import median
import sys
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root=Path(sys.argv[1])
study=json.loads((root/'evidence.json').read_bytes())
fig,(ax,case)=plt.subplots(1,2,figsize=(10.2,4.1),gridspec_kw={'width_ratios':[1.55,1]})
fig.subplots_adjust(left=.075,right=.97,bottom=.18,top=.76,wspace=.30)
fig.patch.set_facecolor('#fafaf7')
fig.text(.045,.93,'What grows after 128 saved CAD edits?',fontsize=20,weight='bold',color='#123335')
fig.text(.045,.843,'Colocad 0.1.1  ·  Linux arm64, 1 vCPU per run, 512 MiB, no swap',fontsize=10.5,color='#3f5d62')
source=study['original']['colocad'][1]['samples_bytes']
late=[s for s in source if s['edit']>=128]
for k,label,color in [('current','Cgroup current','#183d61'),('heapUsed','JS heap used','#ac611d'),('slab','Kernel slab','#167264')]:
    ax.plot([s['edit'] for s in late],[(s[k]-late[0][k])/2**20 for s in late],label=label,color=color,lw=2)
ax.set_title('Release trial 2 · change from edit 128',fontsize=11,loc='left',pad=8)
ax.set(xlabel='Saved geometry edits',ylabel='Change (MiB)',xticks=[128,160,192,224,256])
ax.legend(frameon=False,fontsize=8,ncol=3,loc='upper left')
ax.margins(y=.18)
case.set_title('Persistence replay · same 5,632 members',fontsize=11,loc='left',pad=8)
for i,(suffix,title,c) in enumerate([('-loose','Separate files','#866526'),('-segments','16 segments','#167264')]):
    runs=[v['delta_mib_128_256']['slab'] for k,v in study['layout'].items() if k.endswith(suffix)]
    y=median(runs)
    case.bar(i,y,color=c,width=.49,alpha=.85)
    case.scatter([i-.06,i,i+.06],runs,marker='o',edgecolors='white',linewidths=.7,color='#243b42',s=24,zorder=3)
    case.text(i,max(runs)+.18,f'{y:.3f} MiB',ha='center',fontsize=10,weight='bold')
case.set(ylim=(0,5.35),ylabel='Slab change, edits 128–256 (MiB)',xticks=[0,1],xticklabels=['Separate files','16 segments'],xlim=(-.6,1.6))
for a in (ax,case):
    a.set_axisbelow(True);a.grid(axis='y',color='#d8e0dd',lw=.7)
    a.spines[['top','right']].set_visible(False)
fig.savefig(root/'attribution.png',dpi=155,facecolor=fig.get_facecolor())
