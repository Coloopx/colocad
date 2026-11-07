# Contributing

Start with the [developer docs](docs/README.md), [model formats](docs/concepts.md) and [validation limits](docs/validation.md). Small reproducible models and failing receipts are especially useful contributions.

## Build from source

Use Node.js 22+ and a Rust toolchain supported by the locked workspace dependencies. From the repository root:

```sh
npm run build:kernel
npm test
cargo test --locked -p agent-cad
```

`build:kernel` fetches the pinned SolidCore archive, verifies its hash, builds the current platform's Rust executable and places it in `kernel/native`. The Node tests need that executable or an explicit `AGENT_CAD_BINARY`. They do not rebuild it automatically. A local build only prepares your platform; it is not a complete multi-platform npm release.

When changing the TypeScript kernel SDK:

```sh
npm install --prefix kernel --ignore-scripts
npm run build --prefix kernel
npm test
```

Commit the matching generated `kernel/dist` files with SDK source changes. For harness example changes:

```sh
npm ci --prefix examples/integrations --ignore-scripts
npm test --prefix examples/integrations
```

Those examples deliberately test the pinned **published** package. Use the root/Rust tests for unreleased engine changes; do not assume a passing example test exercised your local engine edits.

## Geometry and performance changes

Add a regression for the geometric behavior, not just an assertion that a file exists. Record expected solids, bounds/volume and relevant internal points. Where feasible, reopen exported STEP using an independent kernel. The [validation script](test/validation/geometry.py) accepts a part artifact directory and `--case manifold` or `--case sensor-rail`; it requires `cadquery-ocp` in your validation environment.

Keep benchmark inputs, environment constraints, raw samples and accounting boundaries with any performance claim. Preserve paired output checks. Do not replace measurements with extrapolated ratios or compare different accepted geometry. See [benchmark reproduction](benchmarks/readme/README.md).

## Documentation changes

Update the relevant guide and link it from the English and Chinese documentation indexes. Keep commands executable from their stated directory. Distinguish feature trees from kernel models, alpha functionality from verified native CAD support, and tool-loop tests from live-model evaluations. Check relative links and run example commands when they change.

## Releases

Update release notes and package metadata together. Build all required native targets, update and verify `kernel/release.json`, and test the packed tarball on each advertised platform. `npm pack` runs `scripts/check-release.mjs`; a clean source checkout must not silently publish a package without its native binaries.

Publishing requires maintainer registry access. Never commit tokens, private `.npmrc` files or third-party proprietary CAD binaries. Native dependency notices live in [NOTICE.md](NOTICE.md) and `kernel/licenses`.

For a pull request, describe the concrete behavior change, the reproduction model and the checks actually run. Call out unsupported cases and environmental gaps instead of presenting a compile-only result as runtime validation.
