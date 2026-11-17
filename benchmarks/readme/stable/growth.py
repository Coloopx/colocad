"""Show late growth on a common scale, subtracting each session's own baseline."""
import json
from pathlib import Path
from statistics import median
import sys

import matplotlib
matplotlib.use('Agg')
from matplotlib import pyplot as plt

root = Path(sys.argv[1])
data = json.loads((root/'engineering.json').read_text())
backends = [('colocad', 'Colocad ' + data['campaign']['label'], '#006a66'),
            ('cadquery', 'CadQuery 2.8.0', '#85541d'), ('build123d', 'build123d 0.13.0', '#534e9d')]
plt.rcParams.update({'font.family': 'DejaVu Sans', 'svg.fonttype': 'none',
                     'svg.hashsalt': 'colocad-paired-revision-growth'})
fig, ax = plt.subplots(figsize=(9.6, 3.45))
fig.subplots_adjust(left=.095, right=.975, bottom=.21, top=.66)
fig.patch.set_facecolor('#f5f6f2')
fig.text(.04, .89, 'Memory growth after edit 128', size=20, weight='bold', color='#192f36')
fig.text(.04, .80, 'Change in current cgroup memory within each session · three trials per backend',
         size=11, color='#425a60')
edits = list(range(128, 257, 4))
for key, label, color in backends:
    curves = []
    for trial in data['sessions'][key]['rounds']:
        current = {s['edits']: s['current_mib'] for s in trial['checkpoints']}
        curves.append([current[i]-current[128] for i in edits])
    points = list(zip(*curves))
    mid = [median(p) for p in points]
    assert mid[-1] == data['sessions'][key]['last_128_growth_mib']
    ax.fill_between(edits, [min(p) for p in points], [max(p) for p in points], color=color, alpha=.13)
    ax.plot(edits, mid, lw=2, color=color, label=f'{label}   {mid[-1]:+.1f} MiB')
ax.axhline(0, color='#596b6d', lw=.8, linestyle='--', zorder=0)
ax.set(xlim=(128,256), xlabel='Saved geometry edits', ylabel='Change (MiB)')
ax.set_xticks([128,160,192,224,256])
ax.grid(axis='y', alpha=.22)
ax.spines[['top','right']].set_visible(False)
ax.legend(loc='lower left', bbox_to_anchor=(0,1.01), ncol=3, frameon=False, fontsize=9, borderaxespad=0)
fig.savefig(root/'revision-growth.svg', metadata={'Date':None})
plt.close(fig)
