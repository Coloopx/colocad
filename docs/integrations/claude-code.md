# Claude Code

Use a project skill with Claude Code's existing Bash and file/image tools. Install colocad **where Claude Code runs**. A local installation is not automatically present in a remote/container session.

## Install in the project

From your project's root (with a `package.json`; create one with `npm init -y` if needed):

```sh
npm install --save-exact @coloopx/colocad@0.1.0-alpha.6
mkdir -p .claude/skills
test ! -e .claude/skills/colocad && \
  cp -R node_modules/@coloopx/colocad/skills/colocad .claude/skills/colocad
```

The command deliberately stops copying if the skill already exists. For an upgrade, review/back up your local changes and replace the whole folder from the newly pinned package. Do not copy only `SKILL.md`.

## Build a part

Start Claude Code in the project. Invoke:

```text
/colocad Make a 100 × 50 × 6 mm mounting plate, with a centered 3 × 2
array of Ø6 through-holes on 32 × 30 mm pitches. Keep the parameters
editable, export STEP, inspect top and iso views, and report the output paths.
```

Claude Code can load a matching skill automatically; explicit `/colocad` makes this first run easy to diagnose. For a shell-only smoke check before inference:

```sh
node .claude/skills/colocad/scripts/build.mjs \
  .claude/skills/colocad/assets/plate-grid.json ./plate-v1 \
  --set length=100 --set xCount=3 --set xPitch=32 --preview
```

The command prints the effective artifact directory. Ask Claude Code to read its `report.json` and open `preview.png` with its image-capable file tool. A successful command alone does not establish visual correctness.

## Add a project convention

If you want CAD behavior on later tasks, merge this short guidance into your existing `CLAUDE.md`:

```text
Use the colocad skill for mechanical CAD. Work in mm and keep the parametric
JSON source with each accepted STEP. Use a new output directory for each build.
Inspect report.json and the required views; identify inferred dimensions.
After edits, persist the accepted model rather than reusing stale parameters.
Return source, STEP and preview paths. Report failed or degraded features.
```

Keep the installed skill's detailed workflow in the skill instead of duplicating it into every prompt. Existing shell/filesystem permissions still apply. The package does not ship a Claude-specific plugin or an MCP server.

Checked against [Claude Code's official skills documentation](https://code.claude.com/docs/en/skills) on 2026-11-07. The copied skill/helper was exercised locally; a Claude Code live-model session was not run in this verification. [Verification scope](verification.md).
