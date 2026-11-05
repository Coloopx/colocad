import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { hand } from '../model.mjs';

const root = process.argv[2];
mkdirSync(root);
const {model} = hand('drive');
function write(name, assembly) {
  const keep = new Set(assembly.map(instance => instance.input));
  for (const feature of [...model.features].reverse()) {
    if (!keep.has(feature.id)) continue;
    for (const key of ['input', 'left', 'right']) {
      if (feature[key]) keep.add(feature[key]);
    }
  }
  const features = model.features.filter(f => keep.has(f.id));
  writeFileSync(join(root, name + '.json'), JSON.stringify({...model, features, assembly}));
}
const link = model.assembly.find(i => i.id === 'Index_Proximal');
if (process.argv.includes('--instances')) {
  const pin = model.assembly.find(i => i.id === 'Index_Pin0');
  function grid(seed, count) {
    return Array.from({length:count}, (_, i) => ({
      ...seed, id:'Part' + i, translation:[(i % 8)*40, Math.floor(i/8)*40, 0],
      rotation_deg:[0,0,0],
    }));
  }
  for (const count of [1, 4, 16, 43]) write('links-' + count, grid(link, count));
  write('pins-16', grid(pin, 16));
}
writeFileSync(join(root, 'hand.json'), JSON.stringify(model));
write('finger', [link]);
