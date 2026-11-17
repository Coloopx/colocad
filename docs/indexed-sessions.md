# Indexed sessions

[中文说明与测量结果](indexed-sessions.zh-CN.md)

Since alpha.9, new SDK and CLI projects use the local Rust SQLite catalog. Build the host executables with `npm run build:kernel` in a source checkout. Alpha.8 projects use a JSON ledger until explicitly migrated.

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

**Source candidate: compact inspections.** SDK sessions with compact storage use a v3 descriptor. A query still executes the kernel and verifies its accepted input. Its full JSON request is content-addressed; each complete reply, including event-specific timings and failures, is checksummed and appended as an independent gzip member. The catalog references the bytes only after they are synced. A crash can leave unreachable trailing bytes, but a later writer cannot overwrite a committed reply. Build/edit artifacts are unchanged. To read a reply without depending on its storage layout:

```js
for (const step of (await session.history({limit:20})).steps) {
  if (step.receiptSha256) console.log(await session.receipt(step));
}
```

Use `ModelingSession.create(dir, {storage:'indexed'})` for the alpha.9 v2 layout with a regular receipt JSON file per step, or `storage:'json'` for v1. Existing projects keep their layout when reopened. Older clients reject v3; use the current CLI/SDK to view or export it. Very large receipts (>32 MiB uncompressed) retain the ordinary file representation. The disk footprint still grows with new information; many distinct requests, geometry edits, or a full in-memory viewer export can grow memory usage. See the [matched experiments](bounded-builds.md).

**Existing projects.** Old JSON projects keep their original format until explicitly migrated. Migration reads one step at a time into private SQLite batches (4,096 rows or about 4 MiB of encoded entries, whichever comes first). A single oversized entry can exceed this target. On Linux, the importer asks the kernel to release consumed input pages and newly synced output ranges; regular catalog operations keep their normal cache behavior. The importer preserves the old manifest under the backup filename printed by the command, and atomically replaces `session.json` after the catalog is durable. Retrying after interruption uses the authoritative manifest. Relative source, receipt, geometry and query paths remain valid after moving the whole directory. Use `ModelingSession.create(dir, {storage:'json'})` for a consumer that requires the old disk format. The compatibility APIs `session.read()` and `colocad history DIR --all` materialize a version 1 ledger in memory. Scripts that directly access `session.json.steps` must use one of these APIs or request legacy storage.

**Storage and recovery.** A v2 `session.json` is a small descriptor pointing to the generated SQLite file in the same directory. Each native writer takes an OS file lock. While migrating it also holds the old writer PID lock. Inputs and receipts are synced before their catalog transitions. Successful geometry artifacts are synced before committing the accepted step. A process killed before completion leaves a running entry that the next writer marks interrupted. Keep the whole closed project directory when copying it. A crash during the final filesystem cutover can leave an unreferenced generation or temporary file. The descriptor selects the committed generation.

**Memory scope.** An indexed query reads a bounded page cache and the selected records. Modeling still needs the current source and geometry. Full viewer export still collects historical meshes. Exact definition spill and streamed assembly STEP export are scheduled separately in [SDD 0006](../specs/0006-bounded-memory-persistence.md). See [M1 validation](../benchmarks/session-catalog/README.md) for measurements and recovery tests.

**Migration budget.** At 500,000 synthetic steps, the Linux arm64 test dropped from 512.00 to 33.06 MiB median cgroup peak. The same migration also completed under a 64 MiB limit. Reserve disk for both the original and the SQLite catalog. Filesystem hints are advisory and the importer still materializes the header and one complete entry. See the [matched experiment and limits](../benchmarks/session-catalog/migration-memory.md).

## Segmented geometry history (development version)

New CLI sessions created by `colocad record` and the recorded Pi integrations use segmented storage. The format is selected when the session is created. The SDK preserves its loose-path default for existing callers. To opt in explicitly:

```js
const session = await ModelingSession.create('./history', {storage: 'segmented'});
const built = await session.build(model, {queryable: true});
if (!built.ok) throw Error(built.error.message);
await session.edit({width: 24});
const delivery = await session.checkout(built.session_step, './original-part');
console.log(delivery.artifactDirectory, delivery.inputFile, delivery.receiptFile);
```

`checkout` creates a new destination and restores one successful geometry revision, including its original STEP, preview, source, receipt, incremental checkpoints and any exact-query snapshot. The caller owns the delivery directory. Pass that directory's `revision` subdirectory to `colocad query` or authorize it in the SDK's `queryRoots`. The equivalent shell command is `colocad checkout ./history s0001 ./original-part`. Use `session.receipt(step)` to read only a receipt.

The accepted head stays loose for incremental modeling. On the next successful build or edit, the catalog streams the displaced head into a segment, syncs the data, commits its directory locators and the new head together, then removes the displaced files. Queries, failed edits and camera observations keep the current head. Segments rotate after 16 revisions or once the previous segment reaches 64 MiB. One large revision can exceed that size. The catalog stores the index location and digest. Each member has its own offset, length and SHA-256. The next writer retries deferred cleanup and overwrites any uncommitted tail on the next append.

In this format, paths in old ledger entries are logical names. For a delivery that must survive another edit, use `checkout`. For tools that open every historical file by its original path, create the CLI session with `--storage compact` or use the SDK default. Existing v1–v3 projects retain their layout when reopened. Copy the whole closed session, including its segments directory.
