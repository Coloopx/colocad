# Alpha.7 query guidance: post-release checks

The [record](../alpha.7-query-guidance.json) distinguishes an **open** grid snapshot defect from a passing ordinary-`holes` example. These targeted macOS arm64 checks are outside the original frozen G1–G3 corpus; they do not rewrite its historical results. Linux reproduction and a native fix remain unverified.

In an empty directory, install the pinned public package with Node 22 or later:

```sh
npm init -y
npm install --save-exact @coloopx/colocad@0.1.0-alpha.7
```

Copy `probe.mjs` and `verify.py` from this directory into that project. Save the JavaScript block in [best practices](../../best-practices.md#run-the-editquery-loop-against-the-installed-package) verbatim as `query-workflow.mjs`. Do not set a native binary override. Run:

```sh
node probe.mjs
node query-workflow.mjs
./node_modules/.bin/colocad replay ./query-session ./query-review.html
```

`probe.mjs` preserves all five receipts in `probe.json`. The two plain build/preview cases pass; all three queryable cases fail with `KERNEL_ERROR`, `BSplineCurve::make_nurb`, status 1001. The finer preview setting does not resolve it. These failures are observations, not expected supported behavior.

The guide then builds the equivalent plate with ordinary `holes`, checks four Ø6 bores, edits to Ø8, rejects the old revision and rechecks centers and volume. Its retained session has four successful steps and one intentional stale-revision rejection. The CLI exports a self-contained HTML replay.

For independent verification, run `verify.py` using Python with the OCP bindings recorded in [occt-guidance.json](occt-guidance.json):

```sh
python verify.py
```

It loads STEP from `probe.json` and the session ledger, checks validity, solid count, envelope and analytic volume, and subtracts the original grid and ordinary-hole plate in both directions. Both material differences were zero within a 1e-6 mm³ threshold. That establishes this alternative's geometry; it does not repair query snapshots for the grid operation. Timings and generated artifact hashes are run-specific.
