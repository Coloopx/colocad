# Codex

Install a project skill and let Codex use its existing terminal and image tools. The agent, harness and native CAD process run in the same sandbox. Install the npm dependency in that environment, including for a remote Codex task.

## Install in the project

From the root of a Node project:

```sh
npm install --save-exact @coloopx/colocad@0.1.0-alpha.9
mkdir -p .agents/skills
test ! -e .agents/skills/colocad && \
  cp -R node_modules/@coloopx/colocad/skills/colocad .agents/skills/colocad
```

Use the whole folder, including `assets`, `scripts` and `references`. Codex discovers repository skills from `.agents/skills`; the current official guide also documents user-level `~/.agents/skills`. This guide uses the project location so installation and versioning are visible to collaborators.

If the target exists, review/back up local skill edits before replacing it with the newly installed version. Do not install a second copy in multiple discovery locations unless you intend to manage that duplication.

## Invoke and inspect

In Codex CLI, use `/skills` to check discovery, or mention `$colocad` explicitly:

```text
Use $colocad to make a 100 × 50 × 6 mm plate with a centered 3 × 2
array of Ø6 through-holes, pitches 32 × 30 mm. Export editable JSON and
STEP, inspect top and iso views, and report artifact paths and assumptions.
```

Attach drawings using your Codex client's normal attachment mechanism. For four views, include a known dimension or scale and state which dimensions may be inferred. If the skill does not appear after a filesystem change, restart the session in the project directory.

Before involving a model, verify the local execution path:

```sh
node .agents/skills/colocad/scripts/build.mjs \
  .agents/skills/colocad/assets/plate-grid.json ./plate-v1 \
  --set length=100 --set xCount=3 --set xPitch=32 --preview
```

The printed `directory` contains the model, STEP, receipt and preview. Codex should inspect the PNG using an available image tool and read the receipt; a file path alone does not show the rendered shape.

## Project instructions

Merge this into your existing `AGENTS.md` if CAD is a recurring task:

```text
For mechanical CAD, use the colocad skill and the project's pinned npm SDK.
Keep units, datums and inferred dimensions explicit. Save editable model JSON,
STEP and the receipt together. Use new revision directories. Check geometry
and relevant views before accepting an edit. Return exact artifact paths.
```

The skill does not change Codex permissions. Use the workspace and sandbox policy already configured by your harness; no blanket permission escalation is needed for this integration.

Skill discovery paths and invocation were checked against [OpenAI's official skill guide](https://developers.openai.com/codex/skills/) on 2026-11-07, after inspecting the local Codex CLI. The helper commands were run locally. This is not a claim of autonomous reconstruction accuracy across unseen drawings. [Verification scope](verification.md).
