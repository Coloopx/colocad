# M1 session catalog acceptance

The candidate implements indexed session storage through the normal SDK, native helper and CLI. Follow the [session guide](../../docs/indexed-sessions.md) to use it. This benchmark exercises durable history and accepted-revision lookup. It uses the [M0 cgroup monitor](../memory/monitor.py) with fresh native Linux arm64 containers, 1 vCPU, 512 MiB, no swap and no network. Node, geometry and catalog children, the sampling process and charged pages count toward the cgroup total. A resident harness and inference are outside this measurement.

```sh
python3 benchmarks/session-catalog/run.py PATH_TO_M0_HISTORY_FIXTURES out/m1-runs --data-directory out/m1-projects
python3 benchmarks/session-catalog/summarize.py out/m1-runs out/m1-report --storage bind-mount
node --test test/session-catalog.test.mjs test/lifecycle.test.mjs test/query-grid.test.mjs
```

Use the pinned image and synthetic ledgers from the [M0 reproduction instructions](../memory/README.md). The data directory must be new and have room for inputs, backup links, indexes and the per-step CAD artifacts. Omitting it uses a new Docker volume. Keep storage configuration fixed when comparing candidates. A bind-mounted filesystem can cache data on its host outside the measured cgroup.

- **10,000 real appends**. A native queryable box build is followed by 100 successful parameter edits and 9,900 successful exact face queries through the SDK. The test checks volumes, query hits, receipt IDs, all 10,001 entries in order, and the accepted geometry after reopening.
- **Synthetic history**. The original 10k, 100k and 500k JSON ledgers are copied in separate preparation containers. Migration and head lookup each run in a fresh cgroup. Each lookup runs three times. A lookup must return the expected last accepted ID. The resource gate is at most 16 MiB median sampled anonymous growth between 10k and 500k. Sampled component peaks can occur at different times. Kernel total peaks use cgroup v2 memory.peak.
- **Recovery and compatibility**. The SDK/CLI tests cover malformed input, SIGKILL during a 50k-row streaming migration followed by retry, termination during a native build, second-writer exclusion, concurrent inspection, backup bytes, relocation, exact query refs, stale revisions, full-ledger export and paging. The extracted npm candidate additionally exercises both storage formats and the actual native executables.

## Recorded results on 2026-11-11

| Synthetic entries | Head total MiB (median of 3) | Head anon MiB (median of 3) | Migration total MiB (1 run) | Migration anon MiB |
| ---: | ---: | ---: | ---: | ---: |
| 10,000 | 21.23 | 13.33 | 41.54 | 18.16 |
| 100,000 | 21.40 | 15.78 | 210.91 | 18.38 |
| 500,000 | 21.03 | 15.73 | 512.00 | 18.45 |

The median sampled anonymous peak changes by +2.40 MiB from 10k to 500k, inside the 16 MiB gate. Each head ID was checked. The alpha.8 study measured 313.23 MiB total at 100k and a V8 heap abort at 500k on the original JSON path. Those earlier results are a [historical baseline](../../docs/memory-persistence-report.md).

The initial build plus **10,000 successful SDK appends** took 294.0 s. The complete cgroup peaked at **271.67 MiB**, sampled anon peaked at **33.12 MiB**. The closed project contains 31,117 files (99.3 MiB logical size, including an 8.0 MiB catalog). Deployment disk capacity and per-step artifact counts matter alongside memory.

The 500k migration reached the 512 MiB cap (492.97 MiB sampled file peak, 18.45 MiB sampled anonymous peak, zero OOM events). Large imports still need headroom for other sandbox processes.

The accepted run used a workspace bind mount. An earlier run on the Docker data volume hit ENOSPC during a receipt write with zero cgroup OOM kills. A development WAL-mode control exposed a read-only snapshot compatibility problem. The final implementation uses SQLite's rollback journal. Read-only mounting, query indexes, relocation, SIGKILL recovery and archive contents were checked alongside the memory measurements. A power-cut hardware test remains open.

[Raw cgroup traces](../../docs/evidence/alpha9-session-catalog/observations.tar.gz) · [Measurements and hashes](../../docs/evidence/alpha9-session-catalog/measurements.json) · [Package and recovery checks](../../docs/evidence/alpha9-session-catalog/acceptance.json) · [Retained development failures](../../docs/evidence/alpha9-session-catalog/development-diagnostics.json).
