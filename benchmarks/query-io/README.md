# Recorded inspections and assembly STEP memory

The alpha.9 candidate records exact queries against an accepted model and uses less memory to write an assembly STEP. Both experiments use the normal SDK, the Linux arm64 executables and fresh cgroup v2 containers. The baseline is commit `e568748`, which already includes indexed session storage. Each measured container has one CPU, a 512 MiB memory limit, no swap and no network. The CAD and monitoring processes share the container. [Measurements and executable hashes](https://github.com/Coloopx/colocad/blob/feat/alpha9-indexed-sessions/docs/evidence/alpha9-query-io/measurements.json) · [Raw traces](https://github.com/Coloopx/colocad/blob/feat/alpha9-indexed-sessions/docs/evidence/alpha9-query-io/observations.tar.gz).

| Workload | Baseline | Candidate |
| --- | ---: | ---: |
| 24 recorded inspections of a 256-hole STEP, median cgroup peak | 117.02 MiB | 82.85 MiB |
| Distinct source bytes referenced by those 25 ledger entries, including the build | 25,945,225 | 1,037,809 |
| Build and export 64 distinct drilled plates, median cgroup peak | 348.62 MiB | 319.66 MiB |
| Same assembly, native process peak RSS reported before artifact publication | 290.49 MiB | 282.70 MiB |
| Same assembly, median end-to-end build time | 32.07 s | 31.73 s |

**Queries.** We imported the CadQuery-generated 256-hole plate from the [M0 part workload](https://github.com/Coloopx/colocad/blob/feat/alpha9-indexed-sessions/benchmarks/memory/README.md) and built its queryable exact snapshot once. Each run used a fresh copy of that closed project. We held the native executable and revision fixed and varied the SDK. All 24 successful inspections returned the same result digest across both implementations, with zero geometry builds, STEP exports or tessellations. Each query still reads and hashes the accepted input using a reusable 64 KiB buffer. Its ledger entry references that input and persists its own request and receipt. The source-byte row sums unique input paths and excludes the database, exact snapshot and receipts. The three baseline peaks were 171.05, 117.02 and 112.79 MiB, compared with 82.85, 83.46 and 82.03 MiB for the candidate. The query series also ran during native compilation, so its wall times include variable host contention.

**Assemblies.** The model contains 64 individually dimensioned plates, each with 16 through holes. We compare the complete builds, including validation, checkpoint generation and delivery of one 32,282,790-byte STEP file. The final timing series used three fresh containers per version, interleaving the old and new packages after compilation finished. Times ranged from 31.81 to 32.09 s on the baseline and 31.71 to 31.84 s on the candidate. The candidate satisfied the geometry and memory checks at similar elapsed time. Every delivered STEP had SHA-256 `553d9847526ffdb3e5b910b007998ef9ee85fe4045ed3d4e21b575facc59789d`. CadQuery/OCCT independently opened the file and checked all solids, bounding boxes, placements, 1,024 cylindrical bore radii and axes, and total material volume of 134,322.246736 mm³. The prepared 16-plate control also preserved its STEP hash and used 124.48 versus 118.98 MiB, one run each.

Encoding records directly into the result string and consuming each parsed definition reduced native process memory. In an intermediate build this alone moved the 64-plate sandbox peak from 348.91 to 347.73 MiB. The trace peaked during output publication. The final build also releases each artifact buffer after writing it and, on Linux, writes final STEP files of at least 4 MiB in 4 MiB chunks. It syncs each chunk and advises the kernel to release its clean pages. At a separate 330 MiB limit, the baseline finished with 72 limit hits, while the candidate finished at 319.73 MiB with zero. Neither run had an OOM. Filesystem advice is best effort. The encoder still constructs the final string in memory and observes the existing 32 MiB STEP limit. Disk-backed exact geometry and larger assembly limits remain open.

## Reproduce

Use the image and fixtures from the [memory study](https://github.com/Coloopx/colocad/blob/feat/alpha9-indexed-sessions/benchmarks/memory/README.md). Set `OLD` to an extracted baseline package and `NEW` to the candidate package, including its Linux arm64 binaries. Set `PLATE` to the absolute path of the STEP produced by the CadQuery 256-hole part case. Run the following from the repository root.

```sh
mkdir -p out/query-seed
docker run --rm --cpus 1 --memory 512m --memory-swap 512m --network none \
  -v "$OLD:/pkg:ro" -v "$PWD/benchmarks/query-io:/bench:ro" \
  -v "$PLATE:/input.step:ro" -v "$PWD/out/query-seed:/seed" \
  colocad-memory:20261111 node /bench/workload.mjs prepare /seed /input.step
python3 benchmarks/query-io/run-query.py "$OLD" "$NEW" out/query-seed out/inspections
python3 benchmarks/query-io/run-assembly.py "$OLD" out/assembly-old
python3 benchmarks/query-io/run-assembly.py "$NEW" out/assembly-new
```

For interleaved timing, use `--counts 64 --repeats 1` with a fresh output directory each time. For the tighter memory case add `--limit-mib 330`. The monitor records cgroup memory.peak, sampled anonymous/file peaks, limit hits, OOMs and elapsed time. The post-build geometry verifier and host filesystem cache are outside the measured scope. These experiments cover CAD subsystem memory and disk use in the tested workloads. The complete resident agent/harness and arbitrary high-complexity B-reps require their own measurements.

The package check builds the bundled four-hole plate, uses one query for through bores and upward faces, relocates the project, edits the row count and diameter, rejects a stale revision, verifies six 8 mm bores and detects modified source. Run it against an installed package using a fresh test directory.

```sh
node test/validation/session-query-cli.mjs node_modules/@coloopx/colocad out/plate-cli
```

The extracted or installed candidate passed on macOS arm64, Linux arm64 and Linux x64. A separate migration check opened a project from the public alpha.8 package, verified the exact v1 backup, repeated its old snapshot query and changed the bore diameter using the candidate. [CLI and skill workflow](../../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell).
