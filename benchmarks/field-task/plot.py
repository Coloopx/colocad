"""Render the published measurement panels from numeric evidence."""
import json,sys
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter,MaxNLocator
data=json.loads(Path(sys.argv[1]).read_text());fig,axes=plt.subplots(2,2,figsize=(11,7.4))
blue,green,orange,gray='#2162aa','#007960','#a4431d','#374151'
def line(ax,x,y,label,color,marker='o'):
    ax.plot(x,y,marker=marker,ms=4,lw=2,color=color,label=label)
def panel(ax,title,xlab):
    ax.set_title(title,loc='left',weight='bold',fontsize=11);ax.set_xlabel(xlab);ax.set_ylabel('MiB')
    ax.grid(axis='y',alpha=.2);ax.spines[['top','right']].set_visible(False)
    ax.xaxis.set_major_formatter(FuncFormatter(lambda x,pos:f'{x:,.0f}'));ax.legend(loc='best',fontsize=8,frameon=False)
ax=axes[0,0];heads=data['head_reads'];line(ax,[r['entries'] for r in heads],[r['peak_mib'] for r in heads],'Fresh process · peak',blue)
ax.set_xscale('log');ax.set_xticks([r['entries'] for r in heads]);ax.set_ylim(0,30);panel(ax,'A   Read latest revision','Indexed metadata entries (log scale)')
ax=axes[0,1]
for k,col in [('alpha.9',orange),('alpha.10',blue)]:
    rows=data['old_writes'][k]['checkpoints'];line(ax,[r['operations'] for r in rows],[r['current_window_mib']['current'] for r in rows],k,col)
ax.set_ylim(bottom=0);panel(ax,'B   Append edits and exact queries','Committed operations (1% edits)')
for ax,key,index,subtitle,xlab in [(axes[1,0],'fixed_geometry','queries','C   Same adapter revision','Additional exact queries'),(axes[1,1],'rebuild','edits','D   Same part count, new STEP each time','Additional geometry revisions')]:
    rows=data[key]['checkpoints']
    for field,name,color in [('current','Entire cgroup',blue),('file','Charged file pages',green),('anon','Anonymous',gray)]:
        line(ax,[r[index] for r in rows],[r[field]/2**20 for r in rows],name,color,'.')
    ax.set_ylim(0,135);ax.xaxis.set_major_locator(MaxNLocator(5,integer=True));panel(ax,subtitle,xlab)
fig.suptitle('What grows when a CAD session gets longer?',fontsize=18,fontweight='bold',x=.074,ha='left')
fig.text(.075,.918,'Colocad on Linux arm64 · 1 vCPU · 512 MiB · no swap. Panels C and D use a KK60 camera adapter.',fontsize=10)
fig.tight_layout(rect=(.025,.06,.995,.90),h_pad=2.2,w_pad=2.5)
fig.text(.075,.038,'A: median of 3 cold processes/size.  B: 4-second pre-commit medians, 1 run/version.  C/D: SDK boundary samples, 1 session each.',fontsize=8)
fig.text(.075,.016,'C and D are fresh cgroups with different starting cache charges; compare changes within each panel. Queries do not rebuild geometry.',fontsize=8)
matplotlib.rcParams['svg.fonttype']='none';matplotlib.rcParams['svg.hashsalt']='colocad-history-1';fig.savefig(sys.argv[2],metadata={'Date':None})
svg=Path(sys.argv[2]);svg.write_text('\n'.join(line.rstrip() for line in svg.read_text().splitlines())+'\n')
