# Legacy ledger migration: isolate and reduce the charged file cache

The original indexed catalog already decoded one JSON step at a time. At 500,000 steps, however, the Linux sandbox reached its 512 MiB limit. In a matched comparison the new importer peaked at **33.06 MiB** (median of three fresh containers). The memory reduction was 93.5%; there was no clear change in elapsed time.

| 500k steps, Linux arm64, 1 CPU, 512 MiB | Initial M1 importer | Bounded importer |
| --- | ---: | ---: |
| Kernel cgroup peak, MiB (median; range) | 512.00 (all three) | 33.06 (33.06–33.13) |
| Sampled file peak, MiB (median) | 492.97 | 14.19 |
| Sampled anonymous peak, MiB (median) | 18.38 | 18.44 |
| Wall seconds (median; range) | 25.63 (24.49–26.02) | 25.44 (24.23–29.53) |
| Limit hits per run (memory.events max) | 1,679–1,752 | 0 |
| OOM kills, all runs | 0 | 0 |

**Scaling and tighter limit.** At 10k and 100k steps, the new importer peaked at 30.63 and 31.75 MiB (one run each under 512 MiB). Under a separate **64 MiB, no-swap** limit, the same 500k input took 23.74 s, peaked at **33.45 MiB**, and recorded zero limit hits or OOMs. The input occupied 454,389,057 bytes; the resulting catalog occupied 529,833,984 bytes. Migration still needs disk capacity for both. The backup is a hard link to the original ledger.

## What changed and why it works

The importer commits after 4,096 rows or approximately 4 MiB of serialized entries, whichever comes first. The SQLite file is private until the header and end of input have been validated; the same atomic descriptor rename publishes the full migration. FULL synchronization and the existing rollback journal remain enabled. On Linux we issue range-specific POSIX_FADV_DONTNEED for consumed input and newly synced, page-aligned database ranges. Each hint covers new bytes; the importer does not rescan the entire file after every batch.

| Same 100k input and private commits; only the advice calls change | Peak cgroup MiB | Sampled file peak MiB |
| --- | ---: | ---: |
| Neither | 211.04 | 189.03 |
| Input only | 126.47 | 105.16 |
| Output only | 113.32 | 92.05 |
| Input and output (production build) | 31.75 | 10.26 |

Each control was measured once. Batching alone did not solve the problem on this mount: both sides of the copy contributed file pages. The hints are advisory. Other filesystems, oversized entries or headers, and index pages revisited by a different key distribution can change the result. Process RAM, charged Linux pages and the host filesystem cache are distinct. These numbers are for metadata migration; they do not establish a complexity bound for arbitrary B-reps, a full agent+harness session or an assembly STEP export.

## Reproduce and check

Use the Linux arm64 image and synthetic alpha.8 ledgers from the [original memory experiments](../memory/README.md). Build the Linux catalog for this revision and for baseline commit `1c8ee5a` with [the native Dockerfile](../../scripts/Dockerfile.native), target `catalog`. The following command launches the production SDK migration with the specified helper in a fresh cgroup. Supply a **new** output directory per run.

```sh
python3 benchmarks/session-catalog/migrate-memory.py "$FIXTURES" out/old-1 "$OLD_HELPER" 500000 --prepare clone
python3 benchmarks/session-catalog/migrate-memory.py "$FIXTURES" out/new-1 "$NEW_HELPER" 500000 --prepare clone --gate-mib 48
python3 benchmarks/session-catalog/migrate-memory.py "$FIXTURES" out/tight "$NEW_HELPER" 500000 --prepare clone --limit-mib 64 --gate-mib 48
node --test test/session-catalog.test.mjs
```

The paired cases used fresh APFS clones on the same workspace bind mount, identical SDK and fixture bytes, native arm64 containers, one CPU, no network and no swap. No VM-wide cache flush was used. If the host lacks clone support, use `--prepare copy` on **both** sides and establish a new baseline. The M0 monitor includes the Node process, helper, monitor process and charged cache. Totals use kernel memory.peak; component maxima are sampled at about 20 ms and need not coincide. Fixture preparation and independent verification are outside the timed cgroup.

After each successful arm64 run we verify SQLite integrity, contiguous sequence count, the accepted head and the byte-identical source backup. Regression tests cover a duplicate ID, invalid format and trailing junk after several commits, and SIGKILL after the staging file contains committed data. A retry starts from the original JSON. The extracted npm candidate also migrated, edited and reopened a real part on macOS arm64, Linux arm64 and Linux x64. The x64 VM additionally migrated 100k entries on a Docker volume under 64 MiB and verified the accepted head (25.55 MiB peak; separate platform and storage, not a paired performance result). An oversized-entry test verifies that a 5 MiB entry remains a single complete history step. A separate pilot ran out of host disk with zero OOMs; it is recorded separately and excluded from successful medians.

[Measurements, hashes and development diagnostics](../../docs/evidence/alpha9-migration-cache/measurements.json) · [Raw traces and compiler-control patches](../../docs/evidence/alpha9-migration-cache/observations.tar.gz) · [Chinese user guide](../../docs/indexed-sessions.zh-CN.md).
