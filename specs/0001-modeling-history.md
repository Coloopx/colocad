# SDD 0001 — Persistent modeling sessions and a human viewer

Status: recorder, viewer, CLI launch/export and integration tests completed. Industrial product and movie acceptance are specified in 0002. Requested 2026-11-07.

## Problem and contract

A harness can generate STEP but a human cannot reliably inspect how an agent reached it. A final assembly animation omits geometry construction, failures and parameter revisions. The same local SDK must persist actual modeling attempts and export an offline interactive history. Agent, harness and CAD stay in one sandbox.

Public additions: `@coloopx/colocad/session` exposes `ModelingSession` and `exportSessionHtml`. `@coloopx/colocad/viewer` exposes a browser custom element `<colocad-viewer>` with a `session` data property. No server or framework is required. Native exact geometry remains authoritative; the browser displays recorded native meshes.

A session is a versioned `session.json` ledger plus immutable per-attempt input/receipt files and native artifact directories. Inputs are cloned before execution. Append attempts before invoking CAD; atomically commit terminal success/failure. Reopening after an interrupted local process preserves the attempt as interrupted. A local lock prevents two writers. A successful later edit never erases a failure. Paths are relative to the session root; relocation is supported. Unknown formats and altered source/mesh/STEP hashes fail explicitly.

`build(model, {label, kind})` records a real native build with preview. `edit(parameters, metadata)` starts from the latest accepted source. `observe(options, metadata)` records an actual cached view and retains its source geometry. The ledger is append-only in sequence; a terminal record is never rewritten. Each entry has measured wall duration, native timings, input digest, status and artifact references. No fabricated memory or inferred speedup is shown.

A browser export embeds data, render code and styles in one HTML file. It supports timeline selection, play/pause/speed, orbit/zoom, iso/front/side/top, exploded display, display cutaway, part visibility and selection, feature/input/receipt inspection, input download and final exact STEP download. Camera, exploded placement and clipping are display-only. Failures stay visible and never inherit a successful badge. Native PNG is the fallback when WebGL is unavailable.

## Acceptance and TDD mapping

| ID | Requirement | Test/evidence |
| --- | --- | --- |
| S1 | Input snapshot, success artifacts and timings survive reopen/move | `test/session.test.mjs` persistence test |
| S2 | Failure and subsequent correction remain visible; edit uses latest successful source | failure/edit test |
| S3 | Concurrent writers refused; dead-owner attempt recovers as interrupted | locking/recovery test |
| S4 | Altered artifacts and traversal refused on export | integrity test |
| S5 | Offline export escapes untrusted labels and bundles native scene data | exporter test |
| V1 | Browser controls change the actual scene; errors shown; no external requests | `test/viewer.browser.mjs` |
| H1 | Pi core/extension use the recorder; scripted tool loop writes history | integration tests |
| H2 | Codex/Claude skill install and helper commands exercised by executable examples | harness smoke example/test; no claim of live inference |
| E1 | Turbofan contains fan, compressor, combustor, turbine, shafts, casing and nozzle | example model test + independent STEP readback |
| E2 | History includes geometry creation/boolean/loft/revolve and assembly milestones | stage contract + native build receipts |
| E3 | Changed fan tip parameter changes measured envelope; holes remain open | native edit + independent material probes |

Write the tests first, record the initial failing command, implement, rerun, inspect the browser, then update docs with the observed scope. No production engine modifications are planned unless a concrete defect blocks this contract.

## Turbofan scope

Original educational two-spool layout, millimetres, axis +Z, editable parameters. Use native revolved shells, cut bores, lofted blades and repeated placed definitions. Target 80–120 named solids and 12–20 recorded milestones. Model a whole engine and expose internals using display clipping, rather than deleting half the engineering model.

The architecture is informed by NASA's [turbofan overview](https://www.grc.nasa.gov/WWW/K-12/airplane/aturbf.html). Dimensions and blade sections are original illustrative choices. This is not a reconstruction of a named engine, a CFD model, or a certified manufacturable design. No aerodynamic, thermal, stress or rotational-clearance performance claims.

## Deferred boundaries

Arbitrary foreign CAD feature-history recovery, live collaborative editing, a background CAD service, persistent autonomous inference and licensed SOLIDWORKS acceptance are outside this change. A user-selected milestone records one real build, not an inferred timestamp for each internal feature. Feature traces retain the kernel's own measurements.

## Scope correction after user review

The synthetic turbofan is retained only as a geometry/rendering fixture, not the primary demonstration of industrial modeling. A future product reconstruction must identify a real model/configuration, document its actual evidence and authoring process, and validate the complete product. Human viewing and offline export are launched by CLI (`view` / `replay`).
