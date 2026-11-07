import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { CadClient } from '@coloopx/colocad';

function requireData(receipt) {
  if (!receipt.ok) throw new Error(JSON.stringify(receipt.error));
  return receipt.data;
}

const outputRoot = resolve(process.argv[2] ?? './cad-output');
await mkdir(outputRoot, { recursive: true });
const model = JSON.parse(await readFile(
  new URL('./node_modules/@coloopx/colocad/skills/colocad/assets/plate-grid.json', import.meta.url),
  'utf8',
));
const cad = new CadClient({ outputRoot, wallMs: 30_000 });
requireData(await cad.validate(model));
const original = requireData(await cad.preview(model, { view: 'iso', chord_mm: 0.2 }));
const top = requireData(await cad.observeArtifact(original.artifact_directory, { view: 'top' }));
const parameters = { ...model.parameters, holeDiameter: 8 };
const edited = requireData(await cad.edit(model, { holeDiameter: parameters.holeDiameter }));
const nextModel = { ...model, parameters };
// The host owns the latest accepted input, so another edit starts from this model.
await writeFile(join(outputRoot, 'current-model.json'), JSON.stringify(nextModel, null, 2) + '\n');
console.log(JSON.stringify({
  original: original.artifact_directory,
  topView: top.artifact_directory,
  edited: edited.artifact_directory,
  currentModel: join(outputRoot, 'current-model.json'),
}, null, 2));
