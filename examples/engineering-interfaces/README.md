# Geometry-backed installation and replacement

This development-source example uses the local Rust SDK; it requires no Python at runtime. Compile this checkout before running. Published alpha.6 binaries predate engineering queries/interfaces.

```sh
node scripts/vendor.mjs
export CARGO_TARGET_DIR="$PWD/../../work/colocad-build/target"
cargo build --release --locked -p agent-cad
export AGENT_CAD_BINARY="$CARGO_TARGET_DIR/release/agent-cad"
AGENT_CAD_BUILD_BINARY="$AGENT_CAD_BINARY" npm run schema
npm install --prefix kernel --ignore-scripts
npm run build --prefix kernel
node examples/engineering-interfaces/run.mjs out/engineering-interfaces
node examples/engineering-interfaces/kk60.mjs out/kk60-interfaces
```

The five flange/support attempts use independently authored OCCT STEP fixtures. A geometric key bore selects the installation plane, circle centers derive an in-plane direction, and the same persistent interface declaration drives assembly. Base, larger housing and arbitrarily rotated sources pass; wrong pitch and missing hole variants fail. There is no case-specific replacement placement. `model.mjs` contains the reusable declarations; `run.mjs` retains actual build/query receipts and exact output hashes.

`kk60.mjs` attaches an independently authored bracket to the complete [catalog-backed HIWIN KK60 reconstruction](../hiwin-kk60/README.md): 82 solids, interface hole correspondence, and three explicit travel positions for bracket/carriage contact, bracket/rail clearance and bracket/motor-support clearance. The latter nine samples are not a continuous-motion or all-pairs collision claim. Smooth nominal holes do not certify M5 threads, loads or OEM manufacturing geometry.

Independent validation uses a separate environment with `cadquery-ocp-novtk==7.9.3.1.1`:

```sh
python examples/engineering-interfaces/verify.py out/engineering-interfaces/runs.json
python examples/engineering-interfaces/verify-kk60.py out/kk60-interfaces/run.json
```

The verifier checks digest, B-rep validity, solid count, volumes, actual transformed hole centers and exact contact/gap/overlap. Analytic fixtures use 1e-5 mm / 1e-7 rad, relative volume 1e-6; catalog reconstruction uses 0.01 mm / 1e-4 rad. Adaptive OCCT volume integration is explicitly selected, since its default quadrature differs on NURBS cylinders. Source packs, hashes, MIT fixture license and uncertainty are in `specs/experiments/0004/fixtures/engineering/manifest.json` and the KK60 `sources.json`.

For human review, record the saved model with preview and start the CLI viewer:

```sh
node bin/colocad.mjs record out/engineering-interfaces/motor-model.json out/interface-session --queryable --label 'Verified flange installation'
node bin/colocad.mjs view out/interface-session
```

Query via `CadClient.query`, `ModelingSession.query`, `cad_query`, or `colocad query ARTIFACT_DIRECTORY request.json --out receipt.json`. The [query guide](../../docs/engineering-queries.md) explains revision identity, relative predicates, partial recognition, interfaces, measurement and native-ref overlays. Snapshots and preview guides stay inside the agent/harness sandbox; no separate worker or geometry service is required.
