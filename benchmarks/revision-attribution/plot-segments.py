"""Plot same-session memory deltas from the last complete matched pair."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

parser=argparse.ArgumentParser()
parser.add_argument('data',type=Path)
parser.add_argument('output',type=Path)
args=parser.parse_args()
data=json.loads(args.data.read_bytes())
pair=max(r['pair'] for r in data['runs'])
fig,axes=plt.subplots(1,2,figsize=(9.6,3.6),layout='constrained')
colors={'baseline':'#975c16','candidate':'#00726c'}
labels={'baseline':'Colocad 0.1.1','candidate':'Segmented history'}
for r in data['runs']:
    if r['pair']!=pair:continue
    samples=[s for s in r['samples'] if s['edits']>=128]
    for ax,key in zip(axes,['slab','current']):
        y=[(s[key]-samples[0][key])/2**20 for s in samples]
        ax.plot([s['edits'] for s in samples],y,color=colors[r['mode']],lw=2,
                label=f"{labels[r['mode']]} ({y[-1]:+.2f} MiB)")
for ax,title in zip(axes,['Kernel slab','Whole sandbox']):
    ax.axhline(0,color='#777777',lw=.6)
    ax.set(title=title,xlabel='Saved geometry edits',ylabel='Change since edit 128 (MiB)',xticks=[128,160,192,224,256])
    ax.grid(axis='y',alpha=.18)
    ax.spines[['top','right']].set_visible(False)
    ax.legend(frameon=False,fontsize=8)
fig.suptitle('KK60 assembly  ·  1 vCPU  ·  512 MiB  ·  final isolated pair',fontsize=13,fontweight='bold')
plt.rcParams['svg.fonttype']='none'
fig.savefig(args.output,metadata={'Date':None})
# Matplotlib leaves spaces before newlines inside path attributes.
if args.output.suffix.lower()=='.svg':
    args.output.write_text('\n'.join(line.rstrip() for line in args.output.read_text().splitlines())+'\n')
