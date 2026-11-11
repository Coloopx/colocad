# Indexed sessions in the alpha.9 development branch

[中文说明与测量结果](indexed-sessions.zh-CN.md)

New `ModelingSession.create()` and `colocad record` projects use the local Rust SQLite catalog. Build the host executables with `npm run build:kernel` in a source checkout. The published alpha.8 package uses the JSON ledger.

```sh
node bin/colocad.mjs record examples/sensor-rail.json ./project --queryable
node bin/colocad.mjs history ./project --limit 20
node bin/colocad.mjs history ./project --after 20 --limit 20
node bin/colocad.mjs migrate ./existing-alpha8-project
```

Pass the returned `nextCursor` as `--after` to request the next page. A null cursor marks the end. The page size defaults to 64 and accepts 1 through 256. A page also targets 1 MiB of encoded entries and includes at least one entry when available. A single large entry can exceed the byte target.

```js
import { ModelingSession } from '@coloopx/colocad/session';

const session = await ModelingSession.open('./project');
const head = await session.latestGeometry();
const { steps, nextCursor } = await session.history({ limit: 20 });
```

The head is the latest successful build or edit. Failed and interrupted attempts, camera observations and exact queries remain available in history. `source()` loads and checks the accepted source file. To inspect an active writer's history from another process, use the CLI history command or `new ModelingSession(directory).history()`. Opening a session for writing checks for interrupted operations.

**Existing projects.** Old JSON projects keep their original format until explicitly migrated. Migration reads one step at a time into private SQLite batches (4,096 rows or about 4 MiB of encoded entries, whichever comes first). A single oversized entry can exceed this target. On Linux, the importer asks the kernel to release consumed input pages and newly synced output ranges; regular catalog operations keep their normal cache behavior. The importer preserves the old manifest under the backup filename printed by the command, and atomically replaces `session.json` after the catalog is durable. Retrying after interruption uses the authoritative manifest. Relative source, receipt, geometry and query paths remain valid after moving the whole directory. Use `ModelingSession.create(dir, {storage:'json'})` for a consumer that requires the old disk format. The compatibility APIs `session.read()` and `colocad history DIR --all` materialize a version 1 ledger in memory. Scripts that directly access `session.json.steps` must use one of these APIs or request legacy storage.

**Storage and recovery.** A v2 `session.json` is a small descriptor pointing to the generated SQLite file in the same directory. Each native writer takes an OS file lock. While migrating it also holds the old writer PID lock. Inputs and receipts are synced before their catalog transitions. Successful geometry artifacts are synced before committing the accepted step. A process killed before completion leaves a running entry that the next writer marks interrupted. Keep the whole closed project directory when copying it. A crash during the final filesystem cutover can leave an unreferenced generation or temporary file. The descriptor selects the committed generation.

**Memory scope.** An indexed query reads a bounded page cache and the selected records. Modeling still needs the current source and geometry. Full viewer export still collects historical meshes. Exact definition spill and streamed assembly STEP export are scheduled separately in [SDD 0006](../specs/0006-bounded-memory-persistence.md). See [M1 validation](../benchmarks/session-catalog/README.md) for measurements and recovery tests.

**Migration budget.** At 500,000 synthetic steps, the Linux arm64 test dropped from 512.00 to 33.06 MiB median cgroup peak. The same migration also completed under a 64 MiB limit. Reserve disk for both the original and the SQLite catalog. Filesystem hints are advisory and the importer still materializes the header and one complete entry. See the [matched experiment and limits](../benchmarks/session-catalog/migration-memory.md).
