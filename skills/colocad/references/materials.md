# Prepare evidence for a mechanical model

Use for a drawing-based part or a specific product. A small dimensioned stock part does not need a large evidence bundle.

## Identify the target and the source

Record the manufacturer/model, variant, optional equipment, units and requested fidelity. An entire actuator is the target; its drawings and BOM describe that target. Do not silently replace it with a generic assembly or a simpler test piece.

Keep an original source URL/file, revision/date, PDF page and checksum when reproducibility matters. Preserve the full page as context, then provide readable detail crops with the view name and crop location. Text extraction helps find dimensions; the rendered page establishes which extension lines, section or component a number belongs to. Do not crop away the datum or section identifier needed to interpret it.

A useful source order is: explicit drawing dimensions and notes, tables for the exact variant, dimensioned sections, then calibrated geometry measurements. A photograph or uncalibrated render supports shape interpretation, not exact dimensions. More views can resolve correspondence but cannot expose every hidden passage or tolerance.

For a PDF, existing Poppler tools can prepare the evidence:

```sh
pdftotext -layout catalog.pdf catalog.txt
pdftoppm -f 25 -singlefile -scale-to 2400 -png catalog.pdf page-25
```

These are material-preparation tools, not requirements of the CAD runtime. Do not add Python/PDF tooling to the modeling sandbox unless the task needs it.

## Separate evidence states

| State | Record | Example |
| --- | --- | --- |
| Specified | Value, unit, source/page/view/annotation | KK60 screw axis height 23 mm, SECTION A-A |
| Derived | Formula and referenced dimensions | Hole stations from rail end and G/pitch |
| Assumed | Value/range, reason and affected claims | Unpublished bearing race dimensions |
| Unknown | Missing quantity and what it prevents checking | Proprietary ball-recirculation path |

Write one shared coordinate convention and the local frame of each part. Name which view corresponds to +/−X, Y or Z; orthographic, perspective and section views are different evidence. Check diameter vs radius, through vs blind, depth origin, units, and normal vs special-order blocks before generating cutters.

Keep manufacturing tolerance and reconstruction acceptance tolerance separate. A 0.01 mm numerical check chosen for your model is not a 0.01 mm manufacturer tolerance. For image-derived dimensions, record calibration and uncertainty. If uncertainty prevents a required interface from being checked, request that specific missing fact while continuing independent parts.

## Minimal handoff to the modeling stage

Provide the selected source pages/crops, dimension table, coordinate frame, interface list, assumptions and acceptance checks. [The product brief](../assets/product-brief.example.json) is a filled example of this evidence convention, not a new executable kernel schema. Do not put its provenance fields into a kernel model with `additionalProperties: false`.
