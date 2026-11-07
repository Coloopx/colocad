# Prompt patterns for agent CAD work

These are adaptable examples, not an instruction to force every task through every stage. Keep the user's product and intended delivery. Separate the task from quoted source evidence. Ask for concise decisions, dimensions and checks rather than a hidden chain of thought.

## Product brief

```text
Use the colocad skill to reconstruct the specified whole product/configuration.
Input evidence: the listed manufacturer pages, drawings, tables and views.
Target: editable source + exact STEP + recorded modeling session + CLI HTML review.
Record units, coordinate datums, required interfaces and which dimensions are
specified, derived, assumed or unknown. Prioritize the interfaces and details
needed for this task. Keep unknown internals explicit; do not substitute a generic
product. Continue independent work if one critical detail is missing.
For each meaningful milestone, build, inspect the relevant view/section and report
measured deviations. Preserve failed attempts. Export the requested movie through
colocad render, with recorded call time separate from playback duration.
```

## Interpret only the evidence needed now

```text
Interpret the supplied overall view and SECTION A-A for the carriage/screw interface.
Return a compact dimension table: quantity, value/unit, evidence state, page/view,
reference datum and uncertainty. Match extension lines to their dimensions.
List conflicting or missing interface values. Do not infer a hidden tolerance from
pixel size. Then identify the local frames and checks needed to model this interface.
```

## Plan one modeling milestone

```text
Using the accepted dimensions and assembly datums, produce the next feature graph.
Name features and repeated part definitions semantically. Express dependent sizes
and placements in parameters. Identify the check that will distinguish a correct
result from a plausible-looking one. Use only installed supported operations.
Validate, build and inspect the result; retain the executable JSON and receipt.
```

## Repair from specific feedback

```text
The accepted revision is the recorded source at the given path/hash.
The attempted change failed at the supplied feature ID with this error/measurement.
Inspect that feature, its operands and local frame. Make a targeted correction;
keep required geometry and dimensional intent. Re-run the affected check before
accepting. If an optional treatment is explicitly omitted, retain its degradation
record and say which claim is weakened.
```

## Check editability or product fit

```text
Test the requested dimension change/pose in the actual CAD model. Recheck the
specified dependent interfaces, hole placement and envelope. Report numerical
results and the source of each target. Render comparison views, then independently
read back STEP when available. Distinguish untested pairs and unknown internals
from passed checks; do not use appearance alone as evidence of fit.
```

## Handoff after a long run

```text
Persist: product/configuration; source identifiers; units and datums; accepted
model path/hash; session path/latest step; parameters and dependency decisions;
passed checks with values; failed attempts; unresolved assumptions; next targeted
check. Keep large meshes and receipts as file references rather than inline text.
```

For performance requests, additionally specify the same input/geometry, output requirements, cold or warm start, preview precision, hardware/container limits and measurement window. Never reuse a construction movie's edited duration as a benchmark.
