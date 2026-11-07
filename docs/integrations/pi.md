# Pi agent core and Pi coding agent

The adapter is tested with `@earendil-works/pi-agent-core@1.5.0` and `@earendil-works/pi-coding-agent@1.5.0`. Pi APIs have changed across releases: pin the versions in your harness and run the adapter tests before upgrading. Older harnesses can use the skill/CLI route without replacing their agent core.

## Pi agent core: register local tools

Clone the example project and install its lockfile:

```sh
git clone https://github.com/Coloopx/colocad.git
cd colocad/examples/integrations
npm ci --ignore-scripts
npm test
```

[pi-tools.mjs](../../examples/integrations/pi-tools.mjs) adapts colocad's JSON Schemas and receipts to Pi `AgentTool`s. [pi-core.mjs](../../examples/integrations/pi-core.mjs) constructs an agent with these tools. Your harness supplies its existing model, stream function and authentication:

```js
import { createCadAgent } from './pi-core.mjs';

// model, streamFn and getApiKey come from your existing harness configuration.
const agent = await createCadAgent({
  model, streamFn, getApiKey,
  outputRoot: '/workspace/cad-output/session-001',
});
await agent.prompt('Build an 80 × 50 × 6 mm mounting plate with four Ø6 holes. '
  + 'Use a centered 2 × 2 grid at 50 × 30 mm pitch. Export STEP and inspect a preview.');
```

This is a host wiring fragment, not a standalone provider login script. Pi core 1.5 requires a `streamFn`. For an existing agent, merge the returned tools into its tools array while idle, retaining your existing tools and context policies:

```js
import { createPiCadTools } from './pi-tools.mjs';
const cadTools = await createPiCadTools({ outputRoot: '/workspace/cad-output/session-001' });
agent.state.tools = [...agent.state.tools, ...cadTools]; // register once; names must be unique
```

The adapter preserves schema `$defs`, serializes CAD operations, throws on failed receipts, and attaches preview PNG content. It does not add a database or copy artifacts outside the sandbox. The harness still owns accepted model state, attachments, token budget and session persistence.

## Pi coding agent: use the extension

From the example project after `npm ci`:

```sh
./node_modules/.bin/pi --extension ./pi-extension.mjs
```

The extension registers five `cad_*` tools. Output defaults to `./cad-output`; set `COLOCAD_OUTPUT_DIR` in the shell launching Pi to choose a trusted absolute destination. Start it from the intended project directory and restart/rebind when changing projects. Use Pi's existing model/auth configuration.

For the modeling workflow as well, load the bundled skill:

```sh
./node_modules/.bin/pi --extension ./pi-extension.mjs \
  --skill ./node_modules/@coloopx/colocad/skills/colocad
```

Use `/skill:colocad` with the part description. The skill may use the shell helper for its starting assets; the extension adds direct typed tools. You can use either route independently.

## Skill-only integration

In a project where colocad and Pi are already installed:

```sh
mkdir -p .agents/skills
test ! -e .agents/skills/colocad && \
  cp -R node_modules/@coloopx/colocad/skills/colocad .agents/skills/colocad
```

Start Pi in that project and invoke `/skill:colocad`. Keep the complete folder: its instructions reference scripts, assets and supporting guides. Review the project skill under Pi's normal project trust flow. No global install or permissions bypass is required.

The example tests execute the actual Pi core loop with deterministic model responses, the actual extension loader, and the published Rust binary. They do not call a live model provider. See [verification scope](verification.md).

References checked 2026-11-07: [Pi core](https://github.com/earendil-works/pi/tree/main/packages/agent), [extensions](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md), [skills](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md). The installed 1.5.0 declarations and CLI help were also checked.
