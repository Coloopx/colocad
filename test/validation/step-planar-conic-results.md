# Planar conic domain regression evidence

The implementation and tests are frozen at
`c8cb5d91a0b67052d2e3a296f4bf0bd5438dff2b`, based on
`c3200d59c326eb848bb76bfef1362e5899c78666`.
This document records existing Windows/WSL2 results from 2026-11-15;
adding it does not change the tested implementation or fixtures.

## Reproduced defect and change

The existing 32 subdivisions can miss an interior extremum of a circle or
ellipse projected onto a plane. A radius-2 arc from 17 to 139 degrees produces
a sampled upper bound of `1.999903617918656`, although its continuous upper
bound is `2`. The unchanged baseline compiled successfully and failed this
specific assertion (runtime exit 101).

The fixed domain includes the stationary points of each projected coordinate,
`C + A cos(t) + B sin(t)`, within the actual bounded arc interval. Existing
samples remain. The change applies only to planar Circle/Ellipse bounds;
other curves and surfaces, geometry, tolerances and validators are unchanged.
This is not a general B-spline or STEP compatibility fix.

## Actual results

| Windows check | Observed result |
| --- | --- |
| Domain regression tests | 7 passed, 0 failed |
| Closed-solid delivery controls | 2 passed, 0 failed |
| Complete workspace Rust runtime | 28 executables; 143 passed, 0 failed, 0 ignored |
| Rust doctests | 0 passed, 0 failed |
| SDK and consumer type checks | Both exit 0; no SDK dist regeneration |
| Complete serial Node runtime | 52 passed, 1 failed; exit 1 |
| Existing independent OCCT WASM reads | Seam input, export and fresh readback: valid, 1 solid, expected volume/bounds |

The Node failure is `killing a streaming migration preserves v1 and retry
discards the partial database`, reporting `SESSION_BUSY` at
`test/session-catalog.test.mjs:155`. Session implementation and this test are
unchanged from the base. The failure is retained; the Node suite is not green.
Compilation and runtime used 2 CPUs, 3 GiB, swap disabled, with no OOM reported.

The seven domain tests cover an extremum between sample parameters, reversed
arcs/loops, a crossing of 360 degrees, full turns, exclusion of out-of-interval
extrema, tilted ellipse projection, a constant projection, and the unchanged
nonplanar path. A shifted UV trim remains rejected.

The public drilled-block fixture changes only its seam parameterization to
17 degrees. Native import/build/export and fresh readback pass. Its negative
control moves one seam vertex by 0.25 mm and is actually rejected with
`GEOMETRY_VALIDATION_FAILED` / `SmBrep::Test #82`. The closed solid is a
nonregression control; it is not claimed to fail on the baseline.

## Acceptance limits and provenance

Independent cylinder identity, five material/air probes and six directed
material cuts were not executed: complete material equivalence remains
unknown. Existing WASM property checks do not establish that equivalence.
Private-source first-checks do not establish acceptance: their recorded
geometry remains invalid, the crosscheck mapping is ambiguous, and candidate
binary/cache attribution requires further diagnosis. No complete private
model repair or completion of the broader import investigation is claimed.

Frozen final source manifest SHA256:
`0680cf98a4fab45796aaca5a19dba729dfe9e40b1b9f2e32b285c707afaad8d6`.
The implementation is delivered as the pinned
`third_party/patches/planar-conic-domain.patch`, SHA256
`081b1f711ab04506336c37c609ea92e868428e52663be8952e2447ffdccddb1e`,
with lock hashes, Docker application and notices updated together.

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
