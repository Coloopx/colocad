# Modeling history: CLI first

For the alpha.9 development checkout, see [indexed sessions and migration](indexed-sessions.md).

Added in alpha.4. In a source checkout, use `node bin/colocad.mjs` in place of `colocad` below. The package entry points are `/session` for the recorder and `/viewer` for the browser component.

```sh
# Build a real milestone and preserve its source, receipt and native preview.
colocad record model.json ./history --label "Bore the hub"

# Repeat record with the next model revision, using the same history directory.
# Launch the local viewer and open the default browser.
colocad view ./history

# Headless sandbox: print the URL; use the existing harness's port forwarding.
colocad view ./history --no-open --port 8787

# Start a display cutaway and also write a standalone offline file.
colocad view ./history --cutaway --export ./review.html

# Export only: no browser and no listening process.
colocad replay ./history ./review.html
```

`view` binds only to `127.0.0.1`; port 0 (the default) chooses an available port. It prints one JSON ready event with the actual URL. Ctrl+C shuts it down. No native GUI or separately provisioned CAD service is required. It serves a snapshot of the recorded session; rerun the command after adding milestones. `--export` writes the same HTML that the viewer serves. Only the HTML root is served, not an arbitrary directory of files.

A session records each actual build/inspection attempt, including failures and interrupted calls. Native artifacts use paths relative to the session. The recorder owns the last successful source so `edit` starts from an accepted model. Archive the whole session directory to preserve all intermediate STEP files. The standalone HTML embeds models, receipts, meshes and previews, plus the final successful geometry STEP.

The viewer supports stage selection, review playback, orbit/zoom, four camera presets, display clipping/explosion, part selection/visibility and input/receipt inspection. These display controls do not change exact CAD. Playback is a presentation cadence; measured call durations are shown separately. With no WebGL, native PNG previews provide a fallback.

## Export an animation from the CLI

```sh
# Optional media dependencies; not needed for CAD builds or the HTML viewer.
npm install -D playwright
npx playwright install chromium
# Install FFmpeg using your host package manager.
colocad render ./history ./modeling.mp4 --width 1280 --height 800 --fps 15
colocad render ./history ./modeling.gif --width 960 --height 600 --fps 10 --seconds-per-step 0.75
```

The renderer reads the saved native meshes and starts Chromium and FFmpeg locally. It displays four synchronized cameras, measured call durations and the construction journal. The final segment explodes and reassembles the saved geometry for inspection. It writes the movie, a `.json` manifest (hashes, timings, frames, source revisions) and a `.poster.png`. Existing movies are not overwritten. Encoding does not invoke the CAD kernel. Video duration is edited review pacing and is **not** a performance benchmark. Rendering uses more memory than a geometry-only sandbox; run it where Chromium and FFmpeg are available.

[Real catalog product: HIWIN KK60](../examples/hiwin-kk60/README.md) includes the generation and independent STEP verification commands.

## SDK integration

```js
import { ModelingSession, exportSessionHtml, createSessionTools } from '@coloopx/colocad/session';
const session = await ModelingSession.create('./history', { title: 'Product reconstruction' });
const result = await session.build(model, { label: 'Main housing', kind: 'geometry' });
if (!result.ok) throw new Error(JSON.stringify(result.error));
await session.edit({ boreDiameter: 8 }, { label: 'Correct bore diameter' });
await exportSessionHtml(session.root, './review.html');
const tools = await createSessionTools(session);
```

Use existing parameters when editing. `createSessionTools` changes `cad_edit` to `{ parameters }`: it uses the recorded accepted source, rather than accepting another model argument. Other modeling calls use their existing arguments. The Pi examples accept `cadSession` or `COLOCAD_SESSION_DIR` to opt into history. See [recorded-pi.mjs](../examples/integrations/recorded-pi.mjs).

For embedding into a host UI, `readSessionBundle(directory)` returns the viewer data. Import `@coloopx/colocad/viewer` in the browser, add `<colocad-viewer>`, and assign that data to its `session` property. This is optional host integration; the CLI remains the end-user entry point.

[Actual modeling workflow and proposed improvements](modeling-workflow.zh-CN.md) · [Specification and test mapping](../specs/0001-modeling-history.md)
