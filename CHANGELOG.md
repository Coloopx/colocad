# Changelog

Notable user-facing changes are recorded here. Colocad remains alpha software; supported geometry and measured workloads define each release's scope.

## Unreleased

- Pack durable query receipts in new sessions; use `session.receipt(step)` for portable readback and `storage: 'indexed'` for alpha.9 file compatibility.
- Release single-use terminal bodies before plain assembly export, preserving the local STEP product and checked world placement.
- Accelerate Level-1 planar loop nesting with conservative UV bounds. [Scope, measurements and reproduction](docs/bounded-builds.md).

## [0.1.0-alpha.9](https://github.com/Coloopx/colocad/releases/tag/v0.1.0-alpha.9) - 2026-11-11

- Add session CLI commands `head`, `inspect` and `edit`. Each recorded exact query verifies and references its accepted input.
- Encode assembly STEP entities directly into the output string and enforce its 32 MiB budget while appending. On Linux, stage final STEP files of 4 MiB or larger in synced 4 MiB chunks and release completed file pages. [Experiment](benchmarks/query-io/README.md).
- Implement a Rust SQLite session catalog, paged CLI/SDK history, streaming migration of alpha.8 sessions and interrupted-write recovery. [Usage](docs/indexed-sessions.md) · [M1 evidence](benchmarks/session-catalog/README.md).

- Reduce the working set of legacy session imports: a 500k-entry migration fell from 512.00 to 33.06 MiB median Linux cgroup peak. [Measurements and limits](benchmarks/session-catalog/migration-memory.md).
- Add reproducible three-backend memory scaling and extreme-limit experiments, raw OOM/timeout/policy outcomes, indexed-history and chunked-project prototypes, and validation/persistence controls. [Report](docs/memory-persistence-report.md) / [中文](docs/memory-persistence-report.zh-CN.md).
- Scope bounded-memory persistence in bilingual [SDD 0006](specs/0006-bounded-memory-persistence.md), update ROADMAP and tuning guidance, and distinguish resident working sets from total geometry/disk complexity.
- Publish the [three-backend release benchmark](docs/benchmark-alpha9.md), the [offline scorecard](docs/benchmarks/alpha9/scorecard.html) and a newly rendered KK60 cover. History migration and paging ship in this version; disk-backed active geometry and streaming a whole assembly STEP remain roadmap items.

## [0.1.0-alpha.8](https://github.com/Coloopx/colocad/releases/tag/v0.1.0-alpha.8) - 2026-11-10

### Fixed

- Exact query snapshots for direct `face_hole_grid` / `hole_grid` stock. Cylinder seam edges contain nested lazy UV lines that the native writer did not materialize. Prepare their exact backing data on the serialization copy, preserving topology identity and support geometry; no mesh/STEP fallback or geometry healing.
- Linux release builds now isolate and lock architecture-specific Cargo caches, preventing concurrent registry extraction races.

### Added and updated

- Publish the expanded English/Chinese modeling best practices, handbook, harness lifecycle, troubleshooting and skill guidance introduced after alpha.7. The executable query/edit example now uses `face_hole_grid` directly.
- Add [CadQuery coverage and design comparison](docs/cadquery-comparison.md), [Chinese report](docs/cadquery-comparison.zh-CN.md) and a machine-readable mapping of all 150 API-index entries. This is a bounded source/contract audit, not full API parity or a new performance result.
- Add [ROADMAP](ROADMAP.md) / [Chinese roadmap](ROADMAP.zh-CN.md): architecture and implementation state, capability gaps, deterministic and live-agent benchmark tracks, and the planned RSI code/tool/skill improvement cycle. G4 runner implementation and campaign acceptance remain pending.
- Rust executor is now 0.5.1. SDK/query schemas are unchanged; updated execution fingerprints invalidate older checkpoints.

### Validation

- 129 Rust tests, 42 Node tests, six deterministic integration tests, schema verification and offline viewer checks.
- Grid regression covers the bundled plate, all six face directions, global grid, seam/loop descriptors, snapshot relocation, parameter/count edits, stale references and CLI replay.
- [Release evidence](docs/evidence/release-alpha.8.json) records the exact three-platform package checks, independent OCCT readback and retained red cases. Historical alpha.7 archives, tag and failure records remain unchanged.

## [0.1.0-alpha.7](https://github.com/Coloopx/colocad/releases/tag/v0.1.0-alpha.7) — 2026-11-09

### Added

- Exact, revision-bound B-rep queries through `CadClient.query`, `ModelingSession.query`, `cad_query` and `colocad query`. Reopened or relocated snapshots preserve identity; edits invalidate old observations.
- Bounded composable topology selection, trim-aware face evaluation, primitive named datums and datum-driven assembly mates with explicit direction and physical residuals.
- Analytic through/blind/counterbored hole recognition, plane-scoped hole groups, declared/recognized `planar_hole_mount/v1` interfaces, all-hole compatibility checks and geometry-driven external STEP replacement.
- Explicit part-pair separation, contact, overlap and containment measurements, plus required checks at bounded pose samples. Read-only queries perform no geometry rebuild, STEP export or tessellation.
- Geometry-reference highlights in recorded CLI/offline review, revision-aware compact agent feedback, capability-gated Pi query tools and updated English/Chinese modeling guidance.

### Fixed

- Required disjoint or tangent cuts now fail with `NO_MATERIAL_REMOVED` instead of silently returning unchanged stock.
- Selection tolerance is validated separately from minimum feature size, permitting the specified `1e-5 mm` query precision.
- Checkpoint reuse requires a source/compiler/target execution fingerprint, preventing an unchanged development version from restoring geometry made under old feature semantics.
- The process-interruption regression now resolves the packaged native when no development override is set.

### Verification

- G1–G3 accepted in their declared domains: 127 Rust tests, 41 Node tests, six deterministic integration tests, offline browser checks, and all 24 frozen geometry cases (18 positives, six correct rejections).
- Independent OCCT readback validates significant exact deliveries, including the 82-solid KK60/accessory reconstruction and geometry-driven foreign replacements.
- Two actual Pi inference tasks run with agent, harness, SDK and native in the same Linux arm64 1 vCPU / 512 MiB cgroup. The product task edits an existing sourced assembly; it does not reconstruct that product from images. Its measured peak is 370.57 MiB with no OOM.
- Fresh archive installations verify the SDK, CLI, packaged native, query/edit/reopen behavior and foreign mounting replacements on macOS arm64, Linux arm64 and Linux x64. Linux checks run offline at 1 vCPU / 512 MiB; x64 uses emulation.

See [G1 evidence](specs/experiments/0004/README.md), [G2/G3 evidence and failures](specs/experiments/0005/README.md), [release archive evidence](docs/evidence/release-alpha.7.json) and the [query guide](docs/engineering-queries.md).

### Compatibility and limits

- This is Colocad's JSON/TypeScript API, not a CadQuery Python or API compatibility layer. Existing modeling entry points remain available.
- Conical, intersecting, side-open and spline-based bore recognition can be partial. Certified allowed contact is bounded to supported opposing planar polyhedral faces; curved-contact certification and region-restricted measurement remain unsupported.
- Semantic intent is re-evaluated after edits. Missing or ambiguous references fail explicitly; arbitrary topology changes do not guarantee one-to-one identity.
- Resource figures apply to the recorded workloads. G4 has not started, and this release establishes no new speed/memory ratio against build123d. Native editable SOLIDWORKS acceptance is unchanged.

## [0.1.0-alpha.6](https://github.com/Coloopx/colocad/releases/tag/v0.1.0-alpha.6) — 2026-11-07

- Documented the SDK → CLI/tools → skill workflow, agent integration examples, CLI visualization and source-backed HIWIN KK60 showcase.
- Improved release verification and publishing integrity checks. Native geometry binaries were unchanged from alpha.4.
- Exact engineering queries and mounting-interface APIs were not included in this version.

Earlier release notes remain in [GitHub Releases](https://github.com/Coloopx/colocad/releases).
