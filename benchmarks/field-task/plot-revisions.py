import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
from matplotlib import pyplot as plt

data = json.loads(Path(sys.argv[1]).read_text())
fig, panels = plt.subplots(1, 2, figsize=(10.5, 4.2), sharey=True)
for ax, mount, label in zip(panels, ['ext4', 'bind'], ['Linux ext4', 'Host directory via virtiofs']):
    ax.axhspan(510, 512, color='#88919d')
    for version, name, color in [('baseline','alpha.10','#a44920'),('candidate','Candidate','#176aa1')]:
        samples = data['trials'][f'final-{mount}-{version}']['checkpoints']
        ax.plot([s['edits'] for s in samples], [s['current_mib'] for s in samples],
                color=color, lw=2.2, label=name)
    ax.set(title=label, xlim=(0,256), ylim=(0,525), xlabel='Saved geometry edits')
    ax.set_xticks([0,32,64,128,192,256])
    ax.set_yticks([0,128,256,384,512])
    ax.grid(axis='y',alpha=.2)
    ax.spines[['top','right']].set_visible(False)
    ax.legend(loc='center right' if mount=='ext4' else 'upper left',frameon=False)
panels[0].set_ylabel('Current sandbox memory (MiB)')
fig.suptitle('KK60 camera adapter · 256 saved revisions',fontweight='bold',fontsize=16)
fig.text(.5,.02,'One serial run per version and mount. Linux arm64, 1 vCPU, 512 MiB, no swap. Each revision includes exact STEP, snapshot and preview.',
         ha='center',fontsize=8)
fig.tight_layout(rect=(0,.07,1,.94))
matplotlib.rcParams['svg.fonttype']='none'
matplotlib.rcParams['svg.hashsalt']='colocad-revision-cache'
dest=Path(sys.argv[2])
fig.savefig(dest,metadata={'Date':None})
dest.write_text('\n'.join(s.rstrip() for s in dest.read_text().splitlines())+'\n')
