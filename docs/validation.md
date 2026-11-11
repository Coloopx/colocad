# Release validation

[Alpha.9 release evidence](evidence/release-alpha.9.json) · [new comparative measurements](benchmark-alpha9.md). The version-specific runs below are historical.

The same public build API compiled the two different feature trees in `examples/`. The manifold has one solid with 27 faces and 54 edges. OCCT read its STEP as a valid solid with volume 45,490.923866 mm³. Reducing the blind-port count from four to three restored 226.194671 mm³, matching π × 3² × 8. The rail has rectangular through-slots patterned along Y. Removing one slot restored 405 mm³. The validation script also probes points within and outside the blind ends, the pocket, and the fillet.

![Manifold precision mesh preview](manifold.png)

The native Windows author compiled against SOLIDWORKS 2024 interop 32.1.0. Runtime creation, feature edit, force rebuild and reopen in the vendor application remain unverified.

The Linux file-format experiment started with a synthesized envelope containing no part geometry or feature tree. On an empty source the pinned Rust encoder refuses synthesis without retained signature tokens. Providing a minimal envelope allowed a cube to round-trip as one solid with six faces. For the actual manifold, its exported STEP contains 54 explicit trim ranges. Of these, 32 are lines delimited by their vertices, 8 are closed full circles and 14 are clamped full-domain NURBS. Checking that the endpoints and natural domains agree permits equivalent vertex-delimited geometry. The experimental re-read then recovered one solid, 27 faces, 54 edges and zero neutral validation findings. This result contains a geometry partition only. Editable fillet selection ownership and native feature-pattern seed references still need to be serialized and independently proven. The CLI intentionally does not release these experimental bytes as an editable part.

The packaged npm archive was installed offline and invoked through the sandbox-local Pi bash tool in an Abelien Linux arm64 runner limited to 1 vCPU and 512 MiB. In the [scoped 0.1.0-alpha.2 run](evidence/scoped-runner.json) the manifold build with preview took 272 ms, the rail 84 ms and the pattern-count edit 181 ms. These are single tool-call wall times. The full cgroup peak includes the offline npm install, cache and files staged on tmpfs. See the JSON receipts in [evidence](evidence/).

We also tested the first two manifold operations with a freshly generated, matching B-rep and a sketch graph compiled directly from the same job. The independent decode of the experimental bytes recovered seven sketch entities across two sketches, one extrusion and one cut. The native operation-class and driving-scalar counts were both zero. Adding the rim fillet to the full graph produced a native encoder refusal at the edge-selection group. The records are in [native-prefix.json](evidence/native-prefix.json) and [native-full-refusal.txt](evidence/native-full-refusal.txt). These are file-format diagnostics, with no claim of successful rebuild in SOLIDWORKS.

The fillet selector separately found all six intended edges in the B-rep recovered for the state immediately preceding the fillet. Both adjacent faces and both endpoints are recorded for each edge. Mapping those input-state identities to persistent SOLIDWORKS operands remains open. In a second diagnostic without the fillet, file readback recovered the linear pattern (four instances along X at 18 mm pitch) and its link to the port-cut seed. Its native operation-class and driving-scalar counts were still zero. See [input-state edges](evidence/historical-selection.json) and [pattern diagnostic](evidence/native-pattern.json).

![Bottom view, showing the four blind bores and the recessed channel](manifold-underside.png)

## npm release 0.1.0-alpha.3

The [archive checks and independent STEP measurements](evidence/release-alpha.3.json) cover macOS arm64, Linux arm64 and Linux x64. Both Linux installations ran offline with 1 vCPU and 512 MiB. The x64 check used an emulated x64 environment and exercised the x64 Node executable and native kernel. Each installation used the packaged CLI to build a manifold and the bundled skill to build a six-hole plate, a sleeve pair and a complete 43-solid hand. OCCT reopened all twelve STEP files. The hand report counted 21 measured instances and 22 translated reuses on each platform.

For a release, stage the three tested executables and their manifests under `kernel/native/<platform>/`. Record the corresponding SHA-256 values and npm version in `kernel/release.json`. The `prepack` check rejects missing executables and hashes that differ from the recorded build. Run `npm pack`, install that tarball in an empty consumer project on each platform, and run `node /path/to/checkout/test/validation/packed.mjs` from that project. Provide the hand model JSON as an optional argument to exercise all 43 occurrences. Publish that tested tarball directly.
