# SDD 0006: bounded working sets and durable projects

2026-11-11 · Target: **0.1.0-alpha.9 (planned)** · [完整中文规格](0006-bounded-memory-persistence.zh-CN.md).

Tracking: [issue #1](https://github.com/Coloopx/colocad/issues/1) · [alpha.9 milestone](https://github.com/Coloopx/colocad/milestone/1). M0 is complete in the [quantitative report](../docs/memory-persistence-report.md); The development branch implements the M1 catalog and session migration through the SDK and CLI. [M1 checks](../benchmarks/session-catalog/README.md) · [usage](../docs/indexed-sessions.md). Geometry spill, streamed STEP, viewer paging and release follow in M2–M5.

Agent, harness, SDK and the local Rust executor remain in the same sandbox. There is no new remote worker architecture. The investigation compares alpha.8 with pinned CadQuery/build123d, including Colocad's own limits, failures and scaling. The Python persistence code in `benchmarks/memory` is an experimental control, not a new production dependency.

## Honest complexity target

Let N be occurrences, D distinct definitions, H history events, Gᵢ each exact definition's size, and W the live dependency frontier. The intended memory bound is runtime/agent overhead + geometry(W) + operation temporaries + fixed index/IO/preview caches + charged file cache. Indexed lookup is O(log N) or O(log H) with a bounded page cache. Disk remains O(ΣGᵢ + N + H); whole-model export must process its output bytes. Arbitrary global Booleans on a large single body cannot promise O(log N) total resident space.

The alpha.8 executor consumes intermediate bodies at last use and shares repeated definitions. It still accumulates final assembly definitions, neutral exchange structures, STEP strings and query artifacts. Local checkpoint/query byte limits are not process limits. `ModelingSession` reads and rewrites the full ledger: O(H) work/space per append and O(H²) cumulative metadata IO. Offline HTML bundles collect all display data. Object counts are not proof of allocator or kernel-arena reclamation.

## Planned architecture

- A versioned disk catalog with bounded page cache, content-addressed geometry/source/mesh objects, append events, indexed accepted head and paginated history/instances. Implement the product catalog in Rust with thin TS SDK/CLI access; preserve the no-Python runtime contract.
- Write/hash/fsync temporary objects, atomically publish, then commit metadata. Crash recovery must not advance accepted head to partial output. Keep failed attempts, validate paths/hashes/revisions, and recover or collect unreferenced objects explicitly.
- Compile a paged dependency plan. Lease geometry by byte budget, spill completed definitions, reload on demand and release last-use inputs. Measure actual arena/allocator residency; use bounded local process lifetimes when required to return memory to the OS.
- Replace retained output byte maps with an artifact sink. Stream definition snapshots, meshes and STEP output using bounded buffers, keeping alpha.8 correctness and stale-reference contracts.
- Keep **chunked project** and **single assembly STEP** as separate deliverables. The latter requires independently verified streaming entity IDs, references, products and rigid placements. A directory of STEP chunks does not satisfy that acceptance item.
- Serve viewer data lazily from CLI. Embedded single-file HTML remains an explicit, budgeted export with an estimate/refusal; the browser still needs memory proportional to its loaded display data.
- Report budget exhaustion by stage with observed/estimated sizes and a recovery point. Never silently omit geometry, lower precision or increase caps to disguise a failure.

Version the large-project protocol separately: the existing full-model JSON, complete artifact hash map and 2 MiB response bound cannot simply be enlarged. New calls page definitions/instances and return a revision, catalog pointer, summary and cursor. Preserve legacy contracts within their declared scope. Keep exact native intermediate snapshots rather than STEP reimports that change naming/selection semantics. A query still needs enough working memory for its selected definition.

Measure assembly validity, mass properties, entity enumeration and export separately. A compositional validity proof over definitions and rigid placements must retain independent output and interface checks; removing required verification merely to fit the cap is not an accepted optimization.

## Milestones and gates

| Milestone | Deliverable | Acceptance |
| --- | --- | --- |
| M0 | Frozen comparative/extreme baselines and indexed/chunked proof | Geometry matched; raw successful and failed runs; correct OOM/timeout/limit classification; independent readback |
| M1 | Rust catalog, SDK migration and pagination | Real 10k appends; separately labelled synthetic 100k/500k metadata probes; bounded head lookup; kill/recovery/migration and query-ref tests |
| M2 | Byte budgets, artifact sink, spill/reload | Frozen G1–G3 and alpha.8 grid regressions; exact output; allocator/cgroup evidence; recoverable budget errors |
| M3 | Large assemblies and streamed single STEP | Beyond old 128-instance boundary; unique/shared definitions, placements/BOM/solids/volume/holes/interfaces independently checked |
| M4 | Lazy viewer and HTML budgets | Long-history open/seek/replay stays within budget; no lost accepted history; structured export-limit errors |
| M5 | Paired verification and release | Native arm64/x64; same geometry, precision and deliverables; held-out families; docs/skill/npm/native released together |

Proposed acceptance targets, to freeze before the production campaign: CAD subsystem 512 MiB, with a separate complete agent+harness test at 2 GiB; 8 MiB initial index and IO budgets each. For fixed-complexity chunked projects, sampled anonymous peak should rise by at most 15% from 128 to 2048 parts. Indexed head lookup from 10k to 500k events should add at most 16 MiB anonymous memory. Disk/time tradeoffs and single-body limitations remain visible. Prototype plateaus alone do not accept M1–M5.

This is one directed G4 development experiment, not delivery of the full RSI runner. Preserve hypotheses and original failures, freeze geometry/verifiers, evaluate correctness and resources before speed, keep unseen product families separate, and retain accept/reject/inconclusive decisions. Benchmark scores do not authorize publishing.
