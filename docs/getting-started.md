# Getting started

Use Node.js 22+ on Linux x64/arm64 or macOS arm64. Installation downloads the npm package; subsequent CAD calls run locally without Python or a Rust compiler.

## Create and edit a mechanical part

Run in a new project directory:

```sh
mkdir my-cad-project
cd my-cad-project
npm init -y
npm install --save-exact @coloopx/colocad@0.1.0-alpha.9
cp node_modules/@coloopx/colocad/examples/manifold.json manifold.json
./node_modules/.bin/colocad plan manifold.json
./node_modules/.bin/colocad build manifold.json ./manifold-v1 --preview
```

`plan` checks and resolves the feature instructions. `build` creates the geometry. Each `colocad build` destination must be new. Read the returned JSON rather than guessing where files were written.

Open `manifold-v1/part/preview.png`. The deliverables are `part/model.step`, the editable `part/feature-tree.json` and `part/report.json`; GLB/STL are display representations. The manifold exercises extrusion, a blind cut, a fillet and a linear pattern.

Create `edit.mjs`:

```js
import { readFile } from 'node:fs/promises';
import { build } from '@coloopx/colocad';

const tree = JSON.parse(await readFile('./manifold.json', 'utf8'));
tree.parameters.instances = 3;
const result = await build(tree, './manifold-v2', { preview: true });
console.log(result.step, result.geometry);
```

```sh
node edit.mjs
```

This version has three bores instead of four. Keep both revision directories and the accepted source. Editing is regeneration from parameters; STEP does not contain the original feature program.

## Use a quick structure or assembly

The skill assets use the more expressive **kernel model** format. They are run by the skill helper rather than the feature-tree CLI:

```sh
node node_modules/@coloopx/colocad/skills/colocad/scripts/build.mjs \
  node_modules/@coloopx/colocad/skills/colocad/assets/plate-grid.json \
  ./plate-v1 --set length=100 --set xCount=3 --set xPitch=32 --preview
```

The command prints a `directory` containing `model.json`, `model.step`, `report.json` and preview files. Run it from the project with the installed dependency so the copied skill resolves the correct SDK version. [Eight starting shapes and structures](../skills/colocad/references/quick-build.md) cover stock, pins, balls, tapers, hole grids, sleeves, brackets and housings.

For a full SDK cycle with mesh reuse, see [sdk.mjs](../examples/integrations/sdk.mjs) and its [run instructions](../examples/integrations/README.md). For an agent, follow [Codex](integrations/codex.md), [Claude Code](integrations/claude-code.md) or [Pi](integrations/pi.md).
