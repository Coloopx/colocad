# Colocad and CadQuery: coverage and design

Audit date: **2026-11-10**, Colocad **0.1.0-alpha.8**. [中文](cadquery-comparison.zh-CN.md) · [Machine-readable mapping](cadquery-coverage.json) · [Roadmap](../ROADMAP.md).

Colocad is an independent agent-oriented mechanical CAD stack. It has useful native modeling, exact queries and geometry-driven assembly, but **does not replace the full CadQuery API or its geometry ecosystem**. It accepts neither CadQuery Python programs nor CadQuery selector strings. Similar operation names do not establish identical options, solver semantics or robustness.

## How to read the audit

The inventory includes all **150 distinct entries** in the requested [CadQuery API index](https://cadquery.readthedocs.io/en/latest/apireference.html). Its source is pinned to [CadQuery commit b3f9afd](https://github.com/CadQuery/cadquery/blob/b3f9afd4f39cbd856d0c46aa30aa37b714b8f6cb/doc/apireference.rst); the JSON records its SHA-256. `/latest/` is mutable and is not a promise about an installed CadQuery version. Constructors, convenience methods and deep geometric operations each count as one index entry; these counts are **not a capability percentage or a differential-test pass rate**.

| Classification | Entries | Meaning |
| --- | ---: | --- |
| Supported domain | 36 | An analogous native operation exists within the stated Colocad contract. Not identical arguments or general equivalence. |
| Partial | 30 | Relevant native capability exists with material option, domain or semantic gaps. |
| Recipe | 29 | A caller can compose primitives or coordinates; no dedicated general equivalent is claimed. |
| Unsupported | 29 | No corresponding public contract. |
| Different abstraction | 26 | Explicit graph/session/typed JSON takes the place of stateful chaining; not drop-in compatibility. |

This is a source/schema audit with links to representative Colocad tests. We did **not** run 150 paired CadQuery/Colocad conformance tests. Low-level Shape/free-function APIs and plugins extend beyond this index and are discussed separately below. The [earlier detailed assembly review](../specs/reviews/0004-cadquery-2026-11-09.md) remains useful for constraint semantics.

## Design comparison

| Dimension | CadQuery | Colocad | Practical consequence |
| --- | --- | --- | --- |
| Geometry stack | Python API over OCCT/OCP | TypeScript/JSON adapter over a local Rust executor and pinned SolidCore dependency | Different kernels and failure domains. Colocad does not use OCCT at runtime; independent OCCT verification is an optional test tool. |
| Modeling interface | Fluent Workplane, Sketch, lower-level Shape and free functions | Explicit parameter/feature graph, final definition IDs and instance graph | More explicit state and machine validation; fewer convenience/low-level APIs. |
| Intent | Relative workplanes, selectors, tags and reusable Python source | Expressions, named topology, recomputable datums/interfaces, required checks | Both can encode intent; neither automatically makes every edit stable. |
| Source editability | Python source is the parametric source | JSON/TS source plus accepted session source and receipts | STEP is geometry exchange, not native editable feature history. |
| Selection | Object/selector APIs and selector DSL | Bounded typed selection expressions, explicit cardinality, partial/ambiguity results | CadQuery code must be translated; Colocad query expressions are not accepted by every modeling feature selector. |
| Snapshot references | Tags/history belong to CadQuery's object/operation model | Immutable native snapshots and revision-bound topology refs | Refs survive reopen/relocation of the same snapshot; an edit requires a new query and semantic recipe resolution. |
| Sketches | Rich face/edge manipulation, constraints and construction workflows | Point/line/circle constraints and bounded local solve; explicit profiles | General sketch offsets, corner editing, interpolating splines and construction state remain gaps. |
| Surfaces | Low-level topology and free-function surface workflows | Bezier patches, bounded networks, bridges, sew and thicken | Colocad's surface operations are useful but do not prove broad surfacing or Class-A parity. |
| Assembly | Hierarchy, selection-based constraints and solve | Hierarchy, shared definitions, datums/attachments, mates, interfaces and pair checks | Constraint names cannot be mapped blindly: CadQuery `Axis` is not automatically a concentric mate. |
| Foreign parts | STEP import and geometric APIs | Registered STEP inputs, exact queries and declared installation recipes | Colocad supports tested compatible replacement families; arbitrary STEP inference remains unproven. |
| Engineering recognition | Users compose topology queries and modeling logic | Supported analytic bores, plane-scoped groups and planar mounting interface profiles | An extra domain layer, not proof that the lower-level CAD API is broader. Complex bores can be partial. |
| Human review | CAD viewers and ecosystem integrations | CLI starts persistent session viewer, offline HTML and optional video render | Colocad records actual steps and query overlays; rendered appearance does not certify fit. |
| Agent integration | Python execution can be wrapped by an agent harness | SDK, CLI, capability-gated tools and packaged skill | Agent, harness and CAD execute together in the sandbox; no additional CAD service is required. |
| Distribution | Python/native dependency environment | npm package with three precompiled native targets; Node 22+ | Installed modeling needs no Rust/Python compiler; Windows/native commercial CAD support remains separate. |
| Performance | Depends on workload, dependency build and execution strategy | Existing matched build123d studies and specific sandbox measurements | There is **no new CadQuery performance result** in this audit, and no universal 10× or memory ratio. |

The CadQuery descriptions above follow its official [API layers](https://cadquery.readthedocs.io/en/latest/primer.html), [design principles](https://cadquery.readthedocs.io/en/latest/designprinciples.html), [assembly constraints](https://cadquery.readthedocs.io/en/latest/assy.html#constraints), [free-function API](https://cadquery.readthedocs.io/en/latest/free-func.html) and [interchange formats](https://cadquery.readthedocs.io/en/latest/importexport.html). In particular, the free-function API provides an alternative without hidden Workplane state; it would be inaccurate to characterize all of CadQuery as a stateful chain. Its documented experimental history is not a guarantee of persistent topology identity across arbitrary edits.

## Important gaps beyond the index

- **Selection-to-feature integration:** query selection supports more compositions than the modeling `EdgeSelector`/`FaceSelector`. A read-only query is not automatically a fillet/shell operand. Extending that bridge needs cardinality, edit and ambiguity tests.
- **Low-level B-rep construction:** no general user-facing Shape/Face/Edge/Wire authoring API equivalent to CadQuery/OCP. Surface patches and imported solids do not cover arbitrary topology construction or repair.
- **Sketch/freeform breadth:** general splines, offsets, text, parametric-function sampling, twist extrusion, advanced sweep options and 2D editing need separate contracts and corpora. Some shapes can be composed manually; that is not API coverage.
- **Interchange/ecosystem:** no full DXF/SVG/XCAF/XBF/3MF/etc. exporter matrix or Python plugin compatibility. General native editable SOLIDWORKS remains incomplete; [native export status](native-solidworks.md) applies.
- **Reliability:** a native operation existing does not establish CadQuery-level success on all imported surfaces, blends or thin walls. Current acceptance is the frozen G1–G3 scope, not a general industrial failure-rate estimate.
- **Engineering certification:** unsupported curved-contact exceptions, continuous-motion checks, threads/fit classes, structural loads, manufacturing tolerances and unknown source accuracy remain separate concerns.

## What is actually verified

[G1–G3 evidence](../specs/experiments/0005/README.md) covers a 24-case frozen corpus, semantic lifecycle/replacement and two live Pi tasks; it does not cover all 150 CadQuery entries or unseen drawing reconstruction. [Alpha.8 release evidence](evidence/release-alpha.8.json) records the new snapshot regression and actual installed bundles. The original alpha.7 failure evidence remains preserved.

The repaired grid path is tested using the bundled four-hole asset, all six stock-face directions and the global grid operation, with native snapshot reopen, seam/loop descriptors and SDK edit/re-query. Independent STEP readback checks geometry rather than trusting native success alone. This is a regression closure, not a new speed benchmark.

Future comparison must separately measure deterministic kernel workloads and agent reconstruction from drawings. Use equivalent accepted geometry, edit challenges, identical budgets and independent reference data. The [roadmap](../ROADMAP.md#benchmark-tracks) defines that work and the guarded improvement loop.

## API-index mapping

The following table groups every indexed entry exactly once. Implementation paths, test links and limits for each group are retained in [cadquery-coverage.json](cadquery-coverage.json). “Supported” always means the bounded analogous contract described above.

<!-- API mapping table is generated from the reviewed coverage JSON. -->
| Group / indexed APIs | Status | Mapping and boundary |
| --- | --- | --- |
| sketch-context: `Sketch`, `Workplane.sketch`, `Sketch.finalize`, `Sketch.copy`, `Sketch.located`, `Sketch.moved`, `Workplane`, `Workplane.center`, `Workplane.moveTo`, `Workplane.move`, `Workplane.placeSketch` | different abstraction | Explicit Profile/Sketch values and orthogonal Frame; caller owns source copies and frames. No mutable Workplane/Sketch chain or stack context compatibility. |
| sketch-state: `Sketch.tag`, `Sketch.select`, `Sketch.reset`, `Sketch.delete`, `Sketch.push` | different abstraction | Named sketch points/circles/constraints and explicit source edits. Not tag-based selection of arbitrary sketch faces or mutable selection state. |
| sketch-topology: `Sketch.faces`, `Sketch.edges`, `Sketch.vertices`, `Sketch.face`, `Sketch.edge`, `Sketch.assemble` | unsupported | No general sketch topology editing API. Solid snapshot queries do not supply an equivalent 2D face/edge editor. |
| basic-profiles: `Sketch.rect`, `Sketch.circle`, `Sketch.ellipse`, `Sketch.polygon`, `Workplane.rect`, `Workplane.circle`, `Workplane.ellipse`, `Workplane.polyline` | supported domain | rectangle/circle/ellipse/polygon profiles. Closed profile construction; not every CadQuery mode, construction geometry or stack overload. |
| profile-recipes: `Sketch.trapezoid`, `Sketch.slot`, `Sketch.regularPolygon`, `Workplane.slot2D` | recipe | Generate polygon or line/arc outline coordinates. Caller must compute the profile; no dedicated constructors or equality claim for every corner case. |
| lines-arcs: `Sketch.segment`, `Sketch.arc`, `Sketch.close`, `Workplane.lineTo`, `Workplane.threePointArc`, `Workplane.close` | supported domain | Outline line/three-point arc segments, or constrained sketch boundaries. Explicit end/via points and closed profiles; no fluent pending-edge behavior. |
| relative-lines: `Workplane.line`, `Workplane.vLine`, `Workplane.vLineTo`, `Workplane.hLine`, `Workplane.hLineTo`, `Workplane.polarLine`, `Workplane.polarLineTo`, `Workplane.sagittaArc`, `Workplane.radiusArc`, `Workplane.tangentArcPoint` | recipe | Compute endpoints/via points in the SDK caller and emit outline segments. Geometric branch and tangency decisions are caller responsibilities. |
| profile-arrays: `Sketch.rarray`, `Sketch.parray`, `Sketch.each`, `Sketch.distribute`, `Workplane.rarray`, `Workplane.polarArray`, `Workplane.each`, `Workplane.eachpoint` | recipe | Caller generates locations/source features; native pattern and shared occurrences cover repeated solids. No callback execution or edge-distribution/stack placement API in the native protocol. |
| sketch-advanced-edit: `Sketch.hull`, `Sketch.offset`, `Sketch.fillet`, `Sketch.chamfer`, `Sketch.clean`, `Workplane.offset2D` | unsupported | No public general 2D wire/face offset, hull, cleanup or selection-based corner editor. Internal profile offsets used by selected solid operations are not a general sketch API. |
| general-splines: `Sketch.spline`, `Workplane.spline`, `Workplane.parametricCurve`, `Workplane.parametricSurface`, `Workplane.ellipseArc`, `Workplane.twistExtrude`, `Workplane.text` | unsupported | No equivalent arbitrary interpolating spline, function-sampling curve/surface, elliptical arc, twist-extrusion or text API. Bezier path/control surfaces are different bounded capabilities. |
| sketch-constraints: `Sketch.constrain`, `Sketch.solve` | partial | Named geometric constraints, local numerical solver, residual and DOF diagnostics. Finite variable/equation budgets and selected point/line/circle constraints; no blanket nonlinear solver equivalence. |
| sketch-mirror-wire: `Workplane.mirrorX`, `Workplane.mirrorY`, `Workplane.wire` | recipe | Reflect profile coordinates and emit a closed outline/polygon. No pending-wire graph or automatic multi-wire assembly semantics. |
| basic-solids: `Workplane.box`, `Workplane.sphere`, `Workplane.cylinder` | supported domain | Native box/sphere/cylinder. Explicit placement and mm units; sphere angular truncation/stack overloads are not claimed. |
| wedge: `Workplane.wedge` | recipe | Extrude an explicit polygon or use a specified loft. No direct wedge signature. |
| booleans: `Workplane.cut`, `Workplane.union`, `Workplane.intersect` | supported domain | Native exact cut/union/intersect on feature IDs. Different kernel; no OCCT glue/tolerance compatibility or universal success-rate guarantee. |
| stack-combine: `Workplane.combine` | recipe | Explicit union graph, or separate named assembly instances. Fusion and assembly are distinct operations; no implicit stack combination. |
| extrusions: `Workplane.extrude`, `Workplane.cutBlind`, `Workplane.cutThruAll` | partial | extrude/draft_extrude/cutout with explicit frame and extent. Up-to-selected-face and automatic through-all semantics require explicit extent derivation; taper/offset restrictions differ. |
| holes: `Workplane.hole`, `Workplane.cboreHole` | partial | holes/face_holes/hole_grid/face_hole_grid and counterbore options. Explicit radius, depth and frame; not every Workplane context/automatic depth overload. |
| countersink: `Workplane.cskHole` | recipe | Subtract explicit cone/cylinder cutters. No dedicated countersink operation or complete countersink recognition contract. |
| loft-sweep: `Workplane.loft`, `Workplane.sweep`, `Workplane.revolve` | partial | profile_loft/loft/sweep/curve_sweep/pipe/revolve. Bounded corresponding sections; planar sweep or one line/arc/Bezier path; no general multi-rail/Frenet/twisted sweep parity. |
| finish-features: `Workplane.shell`, `Workplane.fillet`, `Workplane.chamfer`, `Workplane.split` | partial | Semantic edge selectors, native blends/shells and explicit split tools. Selector families and algorithm success domains differ; unequal-length chamfer and all CadQuery overloads are not established. |
| transforms: `Workplane.rotate`, `Workplane.translate`, `Workplane.mirror` | supported domain | Explicit rigid placement/rotation/translation/mirror. No stack semantics; mirror may produce a new feature or occurrence as specified. |
| center-rotation: `Workplane.rotateAboutCenter` | recipe | Compute/select the intended center and pass an explicit rotation origin. Bounding-box center and mass center must not be conflated. |
| dxf-svg: `Sketch.importDXF`, `Workplane.toSvg`, `Workplane.exportSvg`, `importers.importDXF`, `occ_impl.exporters.dxf.DxfDocument` | unsupported | No public DXF import/export or SVG projection API. PNG/HTML review is not vector drawing interchange. |
| step-import: `importers.importStep` | partial | Registered STEP assets, inspectStep/attachStep/import_step and native product hierarchy. Rigid placements and supported representations only; mixed units/nonrigid mappings and unsupported entities can be rejected. |
| export: `exporters.export`, `Assembly.save` | partial | Exact STEP solids/surfaces and assembly trees; STL/GLB/PNG plus feature source/session HTML. No equivalent full CadQuery exporter format matrix; STEP does not export editable native feature history. |
| stack: `Workplane.all`, `Workplane.size`, `Workplane.vals`, `Workplane.add`, `Workplane.val`, `Workplane.first`, `Workplane.item`, `Workplane.last`, `Workplane.end` | different abstraction | Explicit feature graph, IDs, session accepted source and query result arrays. No Workplane object stack/parent chain compatibility; host array access is not a modeling feature. |
| solid-topology: `Workplane.vertices`, `Workplane.faces`, `Workplane.edges` | supported domain | Revision-bound typed entities and composable query selections. Observed refs are invalidated by edits; query selectors are not automatically accepted by every modeling operation. |
| wire-topology: `Workplane.wires` | partial | Ordered face loops and edge uses. Loops on a solid are not standalone arbitrary wires. |
| body-topology: `Workplane.solids`, `Workplane.shells`, `Workplane.compounds` | unsupported | Definitions/instances expose part results, not these generic topology selections. No public solid/shell/compound entity selector. |
| spatial-selector: `NearestToPointSelector`, `BoxSelector` | unsupported | No direct general nearest-object or bounding-box selector in query expressions. A modeling Near selector and exported AABBs are not equivalent to these selectors. |
| direction-selectors: `BaseDirSelector`, `ParallelDirSelector`, `DirectionSelector`, `DirectionNthSelector`, `PerpendicularDirSelector`, `DirectionMinMaxSelector`, `CenterNthSelector` | partial | Planar-normal predicates, typed rank with projection/datum direction and explicit cardinality. Planar normal support does not cover arbitrary curve tangents; ties retained; semantics differ from CadQuery selectors. |
| metric-selectors: `LengthNthSelector`, `AreaNthSelector`, `TypeSelector` | supported domain | Typed class/range filters and rank by metric. Finite result budgets; tied rank is not silently reduced to one entity. |
| radius-rank: `RadiusNthSelector` | partial | Radius filters/descriptors are available; nth-radius ordering must be implemented by the caller. No native radius rank property. |
| logical-selectors: `BinarySelector`, `AndSelector`, `SumSelector`, `SubtractSelector`, `InverseSelector` | supported domain | Typed union/intersection/difference and bounded all/any/not predicates. Set/target scope is explicit; no Python selector object compatibility. |
| selector-dsl: `StringSyntaxSelector` | different abstraction | JSON selection algebra. CadQuery selector strings are not parsed or accepted. |
| assembly-tree: `Assembly`, `Assembly.add` | supported domain | Named hierarchical instances, shared definitions, rigid placements and STEP product tree. Different construction API and bounded instance/group counts. |
| assembly-constraints: `Assembly.constrain`, `Assembly.solve`, `Constraint` | partial | Named attachments/datums and fixed/coincident/concentric/plane/distance/revolute/prismatic mates. Local solver and explicit direction/residual rules; CadQuery Axis/Plane names do not have identical semantics. |
| appearance: `Color` | partial | Instance colors in Colocad preview/assembly metadata. No OCCT color class or full material/export appearance parity. |
