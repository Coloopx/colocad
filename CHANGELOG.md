# Changelog

Notable user-facing changes are recorded here. Colocad remains alpha software; supported geometry and measured workloads define each release's scope.

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
