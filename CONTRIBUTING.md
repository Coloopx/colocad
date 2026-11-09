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

Commit the matching generated `kernel/dist` files with SDK source changes.

When changing Rust model fields, regenerate the JSON schema and TypeScript types from a freshly built native executable before compiling the SDK:

```sh
cargo build --release --locked -p agent-cad
npm run schema
npm run build --prefix kernel
npm run schema:check
```

The generator honors `CARGO_TARGET_DIR`, or an explicit `AGENT_CAD_BUILD_BINARY`. Runtime tests use `AGENT_CAD_BINARY`; these settings must refer to the same build. The SDK/package and native executor have separate version numbers. For SDD 0004 fixtures, failure logs and independent STEP verification, see the [experiment ledger](specs/experiments/0004/README.md).

For harness example changes:

```sh
npm ci --prefix examples/integrations --ignore-scripts
npm test --prefix examples/integrations
```

The standalone SDK example pins published alpha.3 for compatibility. Current Pi/session adapters import this checkout; query-enabled tests need its matching native. Set `AGENT_CAD_BINARY` to the development executable, or stage it in `kernel/native` with `build:kernel`. Root/Rust tests cover the engine; deterministic integration tests cover the adapter.

## Geometry and performance changes

Add a regression for the geometric behavior, not just an assertion that a file exists. Record expected solids, bounds/volume and relevant internal points. Where feasible, reopen exported STEP using an independent kernel. The [validation script](test/validation/geometry.py) accepts a part artifact directory and `--case manifold` or `--case sensor-rail`; it requires `cadquery-ocp` in your validation environment.

Keep benchmark inputs, environment constraints, raw samples and accounting boundaries with any performance claim. Preserve paired output checks. Do not replace measurements with extrapolated ratios or compare different accepted geometry. See [benchmark reproduction](benchmarks/readme/README.md).

## Documentation changes

Update the relevant guide and link it from the English and Chinese documentation indexes. Keep commands executable from their stated directory. Distinguish feature trees from kernel models, alpha functionality from verified native CAD support, and tool-loop tests from live-model evaluations. Check relative links and run example commands when they change.

## Releases

Update release notes and package metadata together. Build all required native targets, update and verify `kernel/release.json`, and test the packed tarball on each advertised platform. `npm pack` runs `scripts/check-release.mjs`; a clean source checkout must not silently publish a package without its native binaries.

Publishing requires maintainer registry access. Never commit tokens, private `.npmrc` files or third-party proprietary CAD binaries. Native dependency notices live in [NOTICE.md](NOTICE.md) and `kernel/licenses`.

For a pull request, describe the concrete behavior change, the reproduction model and the checks actually run. Call out unsupported cases and environmental gaps instead of presenting a compile-only result as runtime validation.

## Publish a tested archive with the intended identity

Run the kernel, Node, integration and browser checks appropriate to the change, then `npm pack --json`. Keep the prebuilt native manifests/release hashes consistent. For npm publishing, supply `COLOCAD_NPM_TOKEN` through a secret manager or hidden prompt and use:

```sh
node scripts/publish-npm.mjs /absolute/path/to/tested-package.tgz --check-only --expected-user YOUR_NPM_USER
node scripts/publish-npm.mjs /absolute/path/to/tested-package.tgz --expected-user YOUR_NPM_USER --latest
```

The helper isolates npm configuration from project/global credentials, reports the actor, never writes the token to npmrc, publishes the exact archive with scripts disabled, verifies registry SHA-512 integrity, and then updates tags. It refuses a mismatching immutable version. An authenticated `whoami` alone does not establish package write permission; a registry 404 on PUT can still be an authorization issue. The alpha.4 attempt exposed a default-account mismatch; the organization credential was required. Do not replace another project's npm configuration to fix a release.

Keep published Git tags immutable. New skill/tool documentation changes after a tagged release belong to a new release. Attach the verified tarball and link the reproducible modeling artifacts. Never include credentials or local npm configuration in Git or release assets.

A successful upload can be unavailable while npm performs [publish-time scanning](https://github.blog/changelog/2026-07-28-npm-publish-time-malware-scanning-and-dual-use-metadata/). An `E409` mentioning a previously staged version means the version is reserved, not that it is publicly installable. The helper checks public integrity before changing tags and returns `PUBLICATION_PENDING` after bounded polling. Resume the same archive with `--resume --latest`; this never uploads it again. Do not bump a version or replace a tag to escape this state. With npm 11.15+, `npm stage list @coloopx/colocad --json` distinguishes an explicit approval queue; an empty list still does not prove public availability. Explicit staged approval requires the maintainer's 2FA. Keep registry installation verification separate from upload acceptance.
