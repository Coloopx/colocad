# Foreign STEP → exact queries → datum-driven assembly

This checkout example imports a generic OCCT-authored five-hole flange and assembles a shaft using semantic geometry recipes. It is not a commercial product reconstruction or a performance benchmark. Source/license/hash/analytic expectations: [fixture metadata](../../specs/experiments/0004/fixtures/occt-flange.json). [Guide](../../docs/engineering-queries.md) · [中文](../../docs/engineering-queries.zh-CN.md).

Build the current source (the published alpha.6 binaries do not expose this API):

```sh
node scripts/vendor.mjs
cargo build --locked --release -p agent-cad
export AGENT_CAD_BINARY="$PWD/target/release/agent-cad"
npm run schema
npm install --prefix kernel --ignore-scripts
npm run build --prefix kernel
node examples/engineering-queries/run.mjs out/engineering-queries
```

If using a separate `CARGO_TARGET_DIR`, point `AGENT_CAD_BINARY` to its `release/agent-cad`. The script emits the actual artifact directory and query file. Query them through the CLI:

```sh
node bin/colocad.mjs query ARTIFACT_DIRECTORY out/engineering-queries/query.json --out out/engineering-queries/cli-receipt.json
```

The output contains editable `model.json`, immutable exact artifacts, `query.json`, the query receipt and `run.json`. The oversized-shaft negative case must return `ASSEMBLY_COLLISION`. No workflow-side geometric recognition is used: recipes select one cylindrical face and one planar face, and the Rust layer derives and solves their attachments. Assembly seed poses are intentionally perturbed; only the base occurrence is anchored. Rotation around the shaft stays free.

For independent validation, use a separate optional Python environment with `cadquery-ocp-novtk==7.9.3.1.1`:

```sh
python examples/engineering-queries/verify.py out/engineering-queries/run.json
```

The verifier checks STEP hash, valid solids, analytic volumes and the 0.2 mm minimum gap. Python/OCCT is not a dependency of the CAD package. Historical results belong in the [experiment ledger](../../specs/experiments/0004/README.md). General hole recognition, mounting profiles, product replacement and constrained-sandbox acceptance remain separate gates.
