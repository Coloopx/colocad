# Persistent modeling history

Use this workflow when the user wants to inspect construction steps or hand off an offline replay. It requires a colocad build that lists `record` and `replay` in `colocad --help`; the older npm alpha.3 CLI does not include these commands.

Save each meaningful geometry or assembly milestone as JSON, then run from the project:

```sh
./node_modules/.bin/colocad record model.json ./history --label "Bore the hub"
./node_modules/.bin/colocad view ./history
./node_modules/.bin/colocad replay ./history ./history.html
```

`record` accepts a kernel model or a `colocad/1` feature tree, appends a real build with a preview, and prints the receipt. Reuse the session directory for later attempts. A failed build exits nonzero and stays in history. Change the cause, then record a corrected attempt; do not delete the failed step to imply uninterrupted success.

Choose steps that explain geometry: blank → bore/cut → shaped blade → repeated placements → full assembly. Avoid recording only final part placement if the user asked how the kernel constructed the shape. A milestone is an actual build call; internal feature timings remain in the receipt.

The HTML is self-contained and includes orbit/view controls, display-only clipping/explosion, the inputs and receipts, and the final exact STEP. Review playback uses a presentation cadence; recorded call durations are shown separately. Preserve the entire session directory if every intermediate STEP must remain available. A camera change or clipping plane does not modify the saved exact model.

For a modeling animation use the same CLI: `colocad render SESSION ./modeling.mp4` (or `.gif`). This requires optional Playwright/Chromium and FFmpeg. It replays actual persisted geometry in four views, preserving measured call times. Review pacing, cutaway and explosion are presentation only. Do not present the film duration as build time or a benchmark.
