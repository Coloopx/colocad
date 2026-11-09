# SDD 0004 — Engineering queries and geometry-backed assembly interfaces

**Status: Revision 2 supported contracts implemented; G1–G3 accepted within the declared domains/workloads. Development source, not yet included in npm alpha.6.**

Date: 2026-11-09. Baseline: npm alpha.6; source `3c93d2c`. This document defines the proposed contract, not the capabilities of the published package. Normative terms **MUST**, **MUST NOT** and **SHOULD** describe implementation requirements. [中文实施说明](0004-engineering-query-and-assembly-interfaces.zh-CN.md).

The [2026-11-09 CadQuery review](reviews/0004-cadquery-2026-11-09.md) is incorporated here: F01 in §7.2, F02 in §5.6, F03 in §5.5, F04 in §4.2/5.4, F05 in §7.3, and F06 in §10–11. Schemas freeze per implemented layer, not all at M0. [SDD 0005](0005-capability-reliability-and-optimization.md) governs G1 → G2 → G3 → G4 acceptance order. [Experiment and implementation ledger](experiments/0004/README.md) is the authority for actual evidence; this specification alone is not an implementation claim.

## 1. Problem and outcome

An agent can build a part and place an assembly when it already knows every attachment coordinate. It cannot yet reliably ask an unfamiliar part for candidate mounting planes, holes, hole groups and datums, then use those results in the existing assembly solver. Putting that geometry interpretation in each workflow repeats expensive work and binds the workflow to particular parts.

The required path is:

```text
exact B-rep → composable selection → point/plane/axis/frame datums → mates/checks
                       ↓                         ↑
              hole/group recognition → optional mounting-interface profiles
```

Success is replacing an imported component with another compatible component, without changing workflow coordinates or face indices; incompatible or ambiguous replacements must produce useful failures. The SDK and Rust executable remain local to the agent and harness in the same sandbox.

### 1.1 Baseline audit

| Existing implementation | What it establishes | Missing contract |
| --- | --- | --- |
| [CadClient.inspect](../kernel/src/client.ts) | Reads `report.json` | Arbitrary face/edge/engineering queries |
| [inspect-step](../crates/agent-cad/src/step_assembly.rs) | Definitions, instances and placement/template recovery | Geometric interface discovery |
| [name_topology](../crates/agent-cad/src/naming.rs) | Names propagate through supported topology notifications; multiplicity is explicit | Persisted query identity and cross-revision interface resolution |
| [mates](../crates/agent-cad/src/mates.rs) | Local rigid-body solving from explicit attachment frames | Attachment frames derived and checked against actual geometry |
| [incremental checkpoints](../crates/agent-cad/src/incremental.rs) | Optional, bounded reuse; attributed topology is skipped | A durable query snapshot; checkpoints are not that snapshot |
| [KK60 example](../examples/hiwin-kk60/model.mjs) | Detailed geometry and explicitly parameterized placements | Generic assembly of unfamiliar imported interfaces |

SolidCore is derived from SMLib in USD BRep and evolves independently. This work targets the pinned SolidCore dependency, not an assumed upstream capability. SolidCore and OCCT are both B-rep systems. Language choice alone does not establish performance or correctness.

## 2. Scope and completion boundary

**Required for this spec:** bounded composable topology selection; exact descriptors and trimmed-face evaluation; geometry-backed point/plane/axis/frame datums directly consumable by mates; cylindrical-hole recognition in the bounded domain below; planar hole groups; declared/recognized mounting interfaces; versioned references; explicit contact/clearance checks; CLI/SDK/tool access; persisted evidence and regression fixtures. A hole group is not a prerequisite for basic assembly.

**Not required here:** general feature-history reconstruction, universal hole recognition on arbitrary spline surfaces, thread/pitch recognition from nominal bores, automatic inference of manufacturing fits, a new global assembly solver, arbitrary mating-interface standards, mesh-based reverse engineering, SOLIDWORKS output, a remote service, or an autonomous drawing-understanding model.

Completing geometry queries alone is milestone M1, not completion of SDD 0004. Completing this spec is not complete CadQuery/build123d parity.

## 3. Architecture and ownership

| Component | Responsibility |
| --- | --- |
| Rust query module | Load verified exact snapshots; topology, curve/surface descriptors, evaluation, adjacency, geometric/material predicates and recognition |
| Rust datum/interface modules | Derive geometric datums, resolve semantic declarations, validate geometry, produce optional engineering interfaces |
| Existing Rust mate/check modules | Consume resolved frames; solve, report DOF/residuals; run requested exact checks |
| TS SDK | Typed requests/receipts, artifact access and budgets, batch transport, explicit revision selection and persistence |
| CLI | Invoke the same SDK/native contracts; print JSON and optionally save receipts |
| Agent tools and skill | Select intent, give filters, inspect candidates, resolve genuine ambiguities, request checks |
| Viewer | Display selected topology/interface evidence; geometry authority remains native |

Geometry recognition MUST NOT be reimplemented as a different Python/TS workflow. A backend function can be reused or added upstream when it is a general geometry operation; Colocad owns the public contract, engineering semantics, bounds and errors. Domain-specific mounting standards belong in versioned profiles outside the core predicates.

## 4. Queryable revisions and identity

### 4.1 Snapshot creation

Proposed SDK extension: `client.build(model, { queryable: true })`. Existing `build(model)` retains its current behavior and does not acquire query-index or meshing overhead. A queryable build writes immutable exact geometry, topology/name bindings, definition/instance mappings and `query-manifest.json` alongside its ordinary artifacts. It returns `query_revision_id` and the manifest digest.

The manifest binds: query schema/index format versions; executor and kernel revisions; input/model and source-asset hashes; exact geometry hashes; definition IDs/digests; instance paths and rigid transforms; persistent topology IDs and semantic-name bindings; geometry-affecting degradations; recorded geometry tolerance information, including when unavailable. Manifest/snapshot files are included in the artifact hash inventory.

A query snapshot is **not** a preview mesh or the existing opportunistic checkpoint cache. Implementation may reuse the native serializer only after M0 demonstrates named topology and exact geometry survive reload. If additional exact records/sidecars are required, they must be versioned and hash-bound. Kernel arena indices, traversal order and approximate geometric fingerprints alone are not durable identity.

Snapshot creation is atomic. With `queryable: true`, failure to produce a valid snapshot is a failed queryable build, not success with silently missing query support. Existing artifacts are never modified to repair a snapshot. Legacy artifacts require an explicit rebuild from saved source with query support enabled; querying MUST NOT silently rebuild, heal, reimport STEP, tessellate or upgrade a kernel.

The query revision is the SHA-256 of a canonical manifest body that includes payload digests but excludes its own ID, absolute paths, timestamps and timing measurements. M0 freezes canonical encoding and supplies hash vectors. Hash validation covers both geometry and semantic bindings. Referenced payload paths must stay within the artifact root; reject escaping paths and symlink escapes. Relocation changes no identity.

### 4.2 References

```ts
// Proposed public data shape; opaque strings are produced by the implementation.
type TopologyRef = {
  query_revision_id: string;
  definition_id: string;
  definition_sha256: string;
  kind: 'face' | 'edge' | 'vertex' | 'loop';
  topology_id: string;
  instance_path?: string[];
};
```

A reference is valid only in its exact query revision and definition, with the specified occurrence when present. Definitions share geometry; instance paths identify occurrences and distinguish two copies of the same part. IDs MUST survive process restart, directory relocation and queries against the unchanged snapshot. Cross-platform ID equality is not required for independently created snapshots; geometric equivalence is tested separately.

A changed model, source asset, placement map or snapshot/index format produces a new query revision. Raw references are not automatically rebound across revisions, even if an old face looks nearby. Semantic interface declarations are re-evaluated on the new revision and return fresh references and binding diagnostics. The reference outcomes are `resolved`, `missing`, `ambiguous`, `changed_definition`, or `unsupported`; only `resolved` can drive a required mate.

Copy/split/merge notifications can preserve semantic intent, but one name resolving to several faces is an explicit set. A consumer requiring one plane cannot pick the first member. Coplanar fragments can form a composite interface only under an explicit aggregate declaration, validated with actual trims and boundaries.

## 5. Geometry and query contracts

### 5.1 Proposed entry points

```ts
client.build(model, { queryable: true }); // additive overload; retain legacy previousRevision string
client.query(artifactDirectory, request); // Promise<QueryReceipt>
```

```sh
colocad query REVISION_DIR REQUEST.json
colocad query REVISION_DIR REQUEST.json --out NEW_RESULT.json
# Creation via the existing CLI entry points:
colocad build TREE.json NEW_DIR --queryable
colocad record MODEL.json SESSION --queryable
# Internal native entry point, called by the SDK:
agent-cad query --snapshot REVISION_DIR < REQUEST.json
```

One optional agent tool, `cad_query`, accepts `{ artifact_directory, request }`. Register it only when capabilities advertise this query schema. The host selects allowed artifact roots; tool arguments cannot set native executable paths or resource ceilings. Existing `cad_inspect_step` and `inspect()` retain their original meanings.

`cad_build` gains an optional `options.queryable`; `ModelingSession.build`/`edit` gain a queryable option, and `ModelingSession.query(request)` records an observation receipt against the explicitly expected accepted revision without changing accepted source. Queryable session edits retain queryability unless explicitly disabled and no datum/interface depends on it. Models declaring `datums` or `interfaces` opt into query snapshot creation; an explicitly false queryable option with either declaration is rejected. Query observations retain the exact source/receipt hashes; preview-only artifacts redirect through their verified source geometry, never through an unverified path.

Successful CLI execution writes the receipt to stdout and exits 0; failure writes a structured error to stderr and exits nonzero. `--out` also saves the receipt to a new file and refuses overwrite. Querying does not mutate the snapshot. Optional rebuildable caches live in a separate host-managed cache directory.

### 5.2 Batch request

See the parseable [request example](examples/0004-query-request.json). Fields:

| Field | Contract |
| --- | --- |
| `schema` | Exactly `colocad.query/1` |
| `query_revision_id` | Expected manifest identity; mismatch fails before evaluation |
| `target` | Exactly one of `{definition_id}` or `{instance_path}` |
| `space` | `definition` or `assembly`; assembly space requires an instance target |
| `tolerance` | Required positive `linear_mm` and `angular_rad`; geometric comparison thresholds, not manufacturing tolerances |
| `queries` | 1–64 independent requests, each with a unique `id`, `op`, optional typed filters and bounded `limit` |

Queries do not depend on earlier entries within the same batch. One query may contain a bounded selection expression (§5.5), evaluated entirely in the same native invocation. Ordinary selections use the same verified snapshot and target; explicit two-target measurement has its own operation contract. The native process loads each needed definition once per batch. Geometry for repeated occurrences is shared; descriptors are transformed on demand.

For `describe`/`evaluate_face`, every reference must match the revision and target definition; an embedded occurrence path must match the instance target. A definition-only ref may be evaluated on the explicitly selected occurrence. Mixed definitions or implicit occurrence selection fail. Numeric inputs must be finite; ranges are inclusive with `min <= max` and at least one endpoint.

Operations are delivered by milestone, and only implemented operations are advertised:

| `op` | Input beyond common fields | Result |
| --- | --- | --- |
| `faces` | `where` filters, `limit`, optional `cursor` | Face descriptors and references |
| `edges` | `where` filters, `limit`, optional `cursor` | Edge descriptors and references |
| `vertices`, `loops` | Selection/filters, `limit`, optional `cursor` | Vertex and ordered loop descriptors (M1) |
| `select` | A bounded `selection` expression | Typed topology set with explicit cardinality (M1) |
| `datums`, `named_datums` | A supported selection/derivation, or explicit named datum IDs | Point/plane/axis/direction/frame with evidence and unresolved freedoms (M1) |
| `describe` | `refs` (1–256) | Detailed descriptors for explicitly referenced entities |
| `evaluate_face` | Face `ref` and native surface `uv` pair | Position, oriented normal and trim-membership result |
| `holes` | Typed recognition filters, `limit`, optional `cursor` | Recognized holes, uncertain candidates and coverage diagnostics |
| `hole_groups` | Candidate support-face refs and typed pattern filters | Groups on actual support planes, with evidence and unresolved symmetries |
| `interfaces` | Named declaration IDs or a supported profile ID | Declared interface resolutions or recognized interface candidates |
| `measure` | Two explicit owner/occurrence targets, operation and numerical tolerance | Distance, witness and relationship diagnostics (M2; §7.3) |

`faces.where` supports `surface_type`, inclusive `area_mm2` range, `planar_normal_parallel_to`, `planar_normal_aligned_with` and semantic `name`. Direction filters accept a nonzero vector and use `angular_rad`; nonplanar faces do not pass planar-normal filters. `edges.where` supports `curve_type`, `length_mm` range, circular `radius_mm` range and semantic `name`. Type-dependent filters on incompatible geometry types exclude that item; unknown filter names are invalid requests.

`holes.where` supports inclusive entrance `diameter_mm` range, `termination` (`through`, `blind`, `unknown`), `axis_parallel_to` and `opening_on` face references. Diameter means the opening segment's actual diameter; counterbore segments remain separately reported. `hole_groups.where` supports exact `pattern` (`linear`, `rectangular`, `circular`, `unstructured`), `member_count`, and equal-diameter requirement. A supported profile's additional parameters have their own capability-advertised schema. There is no arbitrary expression/eval filter.

### 5.3 Receipt and completeness

The existing `ok/data` and `ok/error` envelope remains compatible. Query errors may add `query_id` and structured `details`; existing callers reading only code/message still work.

A successful query receipt includes `schema`, `query_revision_id`, effective tolerances, target/space, ordered `results`, execution timings (`load`, `index`, `query`, `total`), cache provenance and full receipt/artifact references. Each result has:

- `id`, `op`, `status: complete | partial`, `items`, `diagnostics`;
- `returned_count`, `next_cursor` or null, and a `coverage` object identifying unsupported/uncertain/excluded geometry;
- references and provenance for every engineering claim, including `declared` versus `recognized` origin.

An empty complete geometric selection means no entity met the filter. Empty recognition results with unsupported surfaces MUST be marked partial, not “there are no holes.” Pagination is not incomplete recognition: a complete search may have a `next_cursor`; it still cannot satisfy a uniqueness requirement until the applicable candidate set is complete. Cursors bind revision, target, filters, tolerances, requested space and ordering, and cannot be reused on another query. Ordering is deterministic within a snapshot and does not imply candidate suitability.

Syntax, stale references, integrity or resource failures fail the batch; already completed entries can be retained under error `details.completed_results`, never presented as an overall successful batch. Query items with unresolved engineering meaning are candidates with diagnostics, not transport failures. A required interface with unresolved meaning fails when consumed by a mate.

### 5.4 Exact descriptors

| Entity | Required fields/meaning |
| --- | --- |
| Face | Ref, actual surface type, trimmed area/centroid/bounds, outer/inner loop references, adjacent faces/edges, orientation validity, semantic names |
| Face parameterization | Native UV ranges, periodic axes/periods, seam and singularity information; ranges do not imply trim membership |
| Plane | A point on the underlying plane and oriented unit normal; trimmed face boundaries are separate |
| Cylinder | Axis line, radius, axial interval and angular coverage of the trimmed patch; surface type alone does not mean a hole |
| Other surface | Actual analytic type or `nurbs`/`other`; unsupported specialized parameters are explicit |
| Edge | Ref, actual curve type, trimmed length/bounds, vertices, incident face uses, closed/periodic/seam status; circle adds center/axis/radius and arc coverage |
| Vertex | Ref and point in the requested space, tolerance when known, incident edges |
| Loop | Ref, owning face, outer/inner/unknown classification, ordered edge uses; each use carries edge ref, orientation and face-local trim domain |
| Evaluation | Point and unit normal at native UV, trimmed-domain membership, orientation status; degeneracy/singularities explicit |

A surface centroid is not necessarily inside a trimmed face. If an interior witness is returned it must be separately identified and tested against trims. UV bounds are not a substitute for trim containment. A point in a hole cut out of a face returns `outside_trim`, not a valid face attachment.

Closed, consistently oriented solids permit material-oriented normals. Sheets and invalid/open shells do not acquire a fabricated outward normal. A curved face has a normal field, not one global normal. Unoriented axis lines use a documented deterministic sign convention in definition space; occurrence transforms carry that convention into assembly space without re-canonicalizing against world axes. Hole entry direction is a separate, oriented quantity.

Only rigid transforms are accepted in v1. Positions include translation; directions do not. Local/assembly-space lengths and areas are invariant under rigid placement. Unsupported scaling or reflection is rejected explicitly. Source-unit conversion must follow the existing import contract and be recorded; query output is mm/mm²/mm³ and radians.

Geometric comparison tolerances MUST NOT be silently enlarged to make a match succeed. Report known source/kernel precision and reject an unsupported precision claim. Unknown source accuracy remains unknown, even when numerical predicates pass. Recognition thresholds, manufacturing clearance requirements and solver tolerances are distinct fields.

Loop edge uses are not unique global edges: a periodic seam may use the same edge more than once. Every returned topology reference has a `describe`/traversal consumer. Point projection, if advertised, distinguishes projection onto the underlying surface from the closest point on the actual trimmed face. Invalid UVs, singular normals and trim boundaries have explicit statuses, never fabricated normals.

### 5.5 Composable selection (M1)

One typed expression supports entity enumeration, child topology, ancestors, siblings through an explicit shared child type (`via`), typed filtering, union/intersection/difference, geometric ranking, and cardinality assertions. Predicates support bounded `all`/`any`/`not`; property names and compatible entity kinds are enumerated in the Rust-generated schema. No executable code, arbitrary property lookup or general task DAG is accepted.

Initial expression ceilings are depth 8 and 128 nodes, in addition to §8 traversal/output budgets. Deduplicate entity sets by exact identity; preserve repeated uses inside ordered loops. Ranking declares the measured property (e.g. area or center projection), direction, rank and tie tolerance. A tied rank returns the whole tied set, and a later `expect` requiring one entity fails. Pagination never changes cardinality. Definition-space predicates may refer to an earlier named datum in the same definition; the bounded datum dependency graph must be acyclic.

Persistent declarations and read-only queries share selection semantics. Model recipes use names, typed predicates and relationships, not previous-snapshot refs. The minimum acceptance chain is `selected plane → inner boundary loops → circular edges → circle centers/directions`. A second independent installation plane must not leak its edges into that result. Relative selectors and geometry-derived orientation must follow a rotated imported part; genuinely symmetric geometry remains ambiguous.

### 5.6 Geometric datums (M1/M2)

Add `Model.datums: DatumDeclaration[]` independently of engineering interfaces. A declaration identifies `id`, final part `definition`, verification `tolerance` and a typed `derive` recipe. The output kind follows the recipe (not a separately contradictory field); `selected` contains the bounded geometry `selection` and primitive derivation. Names are unique within their definition. Source geometry must exist and meet cardinality; declarations do not bypass degraded features.

| Kind | Supported foundation and output |
| --- | --- |
| `point` | Selected vertex, analytic circle center, or an explicitly declared design point; distinguish geometric center from a point on material |
| `plane` | Selected analytic planar face: a point on its support plane and oriented material normal where known; not a fully positioned XY frame |
| `axis` | Selected analytic cylinder axis or circle axis: a line point and direction with `unoriented` or explicitly declared direction semantics |
| `direction` | Selected straight edge, or ordered difference of two named geometric points; nonzero and typed as direction, not position |
| `frame` | Origin from a point datum and nondegenerate axis/direction datums; explicit handedness and orthogonality checks |

`Attachment.datum` is an additional alternative to legacy `frame` and engineering `interface`. A plane mate consumes a plane; a concentric mate consumes an axis; point coincidence consumes a point; fixed consumes a complete frame. An internal numerical basis may complete an incomplete datum for evaluation only: its arbitrary axes MUST NOT add constraints or become claimed engineering orientation. Emit derivation/evidence, coordinate space, source revision and unresolved freedoms in receipts.

Supported geometry-derived datums precede hole-group recognition. A shaft, bearing bore or unperforated plate can be assembled without fabricating a `planar_hole_mount` profile. Snapshot observation identity and persistent reconstruction intent remain distinct.

## 6. Engineering recognition

### 6.1 Bounded hole domain

M3 MUST handle analytic cylindrical through holes and cylindrical blind holes with planar bottoms, plus coaxial counterbore steps, in valid closed solids with planar openings. It must recognize equivalent holes split across multiple cylindrical patches or periodic seams. An external shaft, convex boss or unclosed cylindrical recess is not automatically a bore.

Recognition combines exact surface classification, adjacency, trimmed angular/axial coverage, material-side checks and opening/end topology. Coaxial faces merge into one segment only when continuity and boundary evidence support it; equal radius and axis alone must not join disconnected bores. A circle edge alone is insufficient.

A hole descriptor contains an oriented opening frame, all opening-face/edge references, segment radii/axial intervals and supporting face refs, termination classification, depth definition, and recognition evidence. A through hole may have two valid entry choices; its physical feature is not double counted. Counterbore diameter/depth are distinct from the bore. A blind-hole depth is measured from the selected opening plane to the planar bottom along the inward axis.

Intersecting bores, partial side openings, nonplanar entrances, conical drill tips, countersinks, arbitrary NURBS-equivalent cylinders and thread form recognition may return unsupported/partial candidates in v1. They MUST NOT be falsely classified as simple through/blind holes. Nominal holes do not imply a thread designation, pitch, tolerance or press fit. New recognizers extend a versioned capability rather than silently broadening semantics.

### 6.2 Hole groups

A group is a set of recognized openings on a physically verified common support plane, with compatible axes. Grouping uses distances and angles in that plane, not screen/world bounding boxes. Returned data includes member refs, center locations, pairwise distances, inferred pattern parameters, maximum fit residual, competing groupings and symmetry classes.

Rectangular patterns report both pitches and counts; circular patterns report center, radius and angular spacing; linear patterns report axis and pitch. A repeated-looking subset does not silently exclude extra holes: return competing candidates and list excluded members. Tolerance-near ambiguities are retained. Partial search/recognition cannot establish a unique pattern.

### 6.3 Declared and recognized interfaces

Self-authored geometry SHOULD declare design interfaces using stable feature/name relationships. Imported geometry uses recognition where design intent is unavailable. Both resolve to the same output contract, but provenance remains visible and declarations are checked against the final geometry. Declarations cannot manufacture missing planes/holes or bypass a failed required feature.

Initial optional profile: `planar_hole_mount/v1`. It groups geometric datums for one supported mounting plane (or an explicitly declared coplanar aggregate), one hole group, and optionally a coaxial register or pilot. Candidate recognition is per definition and reusable across occurrences. The profile does not automatically infer bolt size, thread, load capacity or standard compliance, and is not required for raw geometric datum mates.

A resolved interface contains: `name/id`, owner definition/digest, supporting topology/holes, explicit datum origin rule, plane normal, optional locating axis, optional in-plane orientation, actual dimensions, applicable tolerances, provenance, evidence refs and unresolved freedoms. Origin rules are `hole_group_center`, `axis_plane_intersection`, or a declared geometric datum point. A numeric legacy frame alone is not evidence of a recognized mounting interface.

A plane fixes neither origin nor in-plane rotation. A plane and coaxial axis still leave rotation. Symmetric hole patterns may admit several discrete orientations. Candidates MUST expose continuous freedoms and discrete symmetry alternatives separately. An agent must select a supported datum/key direction or explicitly permit an equivalent orientation; the engine must not invent a unique orientation from topology ordering.

## 7. Model and mate integration

Proposed additive model field: `interfaces: InterfaceDeclaration[]`. Each declaration has `id`, `definition` (an output feature used as a part definition), `profile`, `selectors`, `datum` rules and `requirements`. IDs are unique within a definition. Selectors use semantic names, typed geometry predicates and explicit cardinality. V1 model declarations do not accept snapshot topology refs as persistent selectors: adding a mate changes the model/query revision, so carrying those refs into the new model would already make them stale. Exact refs remain valid observation/evaluation inputs on their original snapshot.

The [declaration fragment](examples/0004-interface-declaration.json) fixes the v1 shape. Its fields are:

| Field | v1 meaning |
| --- | --- |
| `selectors.plane` | Face `where` predicate and required cardinality; default profile requires exactly one face |
| `selectors.hole_group` | Group predicate/cardinality; `opening_on: "plane"` binds the openings to the resolved plane slot, not an arbitrary sibling expression |
| `selectors.register` (optional) | Cylindrical-face predicate/cardinality plus required `material_side: internal | external`; its axis must meet the mounting plane |
| `datum.origin` | `hole_group_center`; `axis_plane_intersection` with `axis_slot: "register"`; or `declared_point` with a bounded scalar `point` on the support plane |
| `datum.normal` | `outward` in this closed-solid profile; flipping it for a mate is not part of recognition |
| `datum.x_axis` (optional) | `{kind: "orientation_datum", name}` explicitly selects one of the declaration's named orientation datums |
| `orientation_datums` (optional) | Unique named references to geometry-derived direction datums, or explicitly declared numeric directions in definition coordinates; nonzero and in-plane. Numeric intent is labelled and not falsely claimed to follow a changed source geometry. |
| `requirements.verification_tolerance` | Positive linear/angular comparison thresholds; separate from manufacturing fit requirements |
| `requirements.pattern` | Declared supported pattern dimensions/counts; rectangular pitch/count order is relative to the selected orientation datum, or reported as unresolved if no unique axes exist |
| `requirements.bore_diameter_mm`, `termination` | Required actual opening bore diameter and termination for all selected members |

For declarations with no selected orientation datum, permutation-invariant pattern dimensions may be checked as a set, but a directional pitch requirement cannot silently acquire axes. A composite coplanar plane must explicitly set an aggregate selector and require a connected mounting region with defined trims; it is never inferred by relaxing cardinality. The initial recognizer may return a composite candidate as unsupported until this aggregate path is implemented and tested.

A datum origin may lie in the empty center of a mounting bore or pattern; it is a point on the support plane, not necessarily a material/contact witness. Report its trim membership separately. Only a surface-evaluation/contact claim requires the evaluated point to belong to the actual trimmed face. Declarations without a complete orientation can remain candidates until an attachment supplies an explicit orientation; required unresolved constraints fail before the numerical solve.

Declarations are evaluated against final, undeformed part geometry after features and before mate solving. Fields depending on model parameters use the existing bounded scalar-expression language. Portability across edits comes from re-evaluating declared intent and checking requirements, not from preserving an integer face ID. A source asset change invalidates exact bindings and triggers explicit semantic re-resolution or a changed-definition error.

Existing attachment form remains supported:

```json
{"instance":"motor", "frame":{"origin":[0,0,0],"x_axis":[1,0,0],"y_axis":[0,1,0]}}
```

Proposed alternative form:

```json
{"instance":"motor", "interface":"mount", "orientation":{"kind":"datum","name":"connector_up"}}
```

Exactly one of explicit `frame`, `datum` and `interface` is allowed; omitted selection retains the legacy default frame only for legacy attachments. `instance` keeps the current instance-ID namespace; datum/interface lookup is scoped to that instance's definition. `orientation` is required only when the requested mate needs orientation not uniquely defined by its datum. The model form selects a declared datum with `{kind: "datum", name}`. Supplying no orientation never means “choose the first.”

Reported alternatives have revision-bound IDs for inspection and evidence, not persistent mate selectors. If an agent chooses one, it must express that choice as declared orientation intent in definition coordinates, retain its source receipt, and rebuild/revalidate the declaration and its geometric requirements. The new result gets new refs. A numeric direction alone does not establish correspondence to a connector or key. Changing or rotating source geometry requires fresh semantic resolution; no automatic carry-over of the old alternative ID is allowed.

Resolution produces a frame in definition coordinates, then applies the same nested placement chain used by the existing assembly implementation. The interface's reference plane and origin are oriented so +Z points out of the mating surface; the declared X direction and `Y = Z × X` form a right-handed frame. A hole's inward drilling direction remains separate. Resolution errors precede the mate solve.

Legacy defaults for fixed/coincident/concentric/plane/revolute/prismatic/distance remain unchanged. Additive direction policies are specified in §7.2. A local solver result is not a proof of a globally unique assembly. Results expose physical positional/angular residuals separately from weighted solver residuals, remaining continuous DOF, discrete alternatives and failed constraints. Expectations for free motion must be declared. No silent substitution of fixed for revolute/prismatic, and no fixture-specific translation fallback.

### 7.1 Interface compatibility and fit checks

A fully specified frame can place incompatible parts; placement success is not compatibility. Add an explicit, versioned assembly check `interface_compatibility` referencing two named attachments. It checks the declared mounting-plane relation, required register/axis alignment and the caller's supported pattern matching policy. Return the measured hole-center/axis/radius deviations, correspondence, discrete alternatives and pass/fail thresholds.

The [check fragment](examples/0004-interface-check.json) uses `a`/`b` named attachments, `plane_relation: opposed | aligned`, numerical predicate `tolerance`, and independent engineering `requirements`. The v1 correspondence form is `{kind: "pattern", require_all: true, symmetry: "require_unique" | "allow_equivalent"}`. It searches a bijection preserving the supported group's in-plane geometry. `require_unique` rejects multiple geometrically valid correspondences; `allow_equivalent` explicitly permits the reported alternatives. All members are required; matching a convenient subset is not an accepted interface check.

Matching validates geometry after the mate transform and does not move parts itself. Persist the resolved member pairs as revision-bound result evidence, not as the next model's selectors. Explicit per-hole correspondence in model source is deferred until a semantic member-label contract exists; raw hole IDs from an earlier snapshot must not silently become persistent bindings. Optional register requirements specify a radial clearance interval and maximum axis offset, not inferred fit classes.

`max_plane_gap_mm` and `max_hole_axis_offset_mm` are nonnegative physical acceptance limits. The numerical tolerance bounds predicate evaluation and must be reported separately. `hole_size.kind: independent_ranges` checks each side's inclusive diameter range; it does not assert bolt/thread compatibility. A required thread/fit claim without corresponding evidence is unresolved, never a pass. Persist the resolved correspondence and references with the assembly receipt.

Requirements distinguish clearance bores, nominal threaded holes and mating registers; unequal diameters are not automatically incompatible. The caller provides size/clearance rules and any known engineering standard. Missing thread/fit evidence remains unresolved. Any-equivalent orientation is opt-in and does not prove external clearance in every equivalent pose.

Exact interference/clearance checks use actual transformed B-reps and explicit part pairs, allowed contact regions, pose samples and tolerances. Contact at an intended mounting face is not automatically interference. Required check failures keep the attempt as failed in the modeling session and do not replace the accepted source. Do not add an unconditional all-pairs Boolean pass to every assembly.

Envelope/simplified representations and feature degradations remain visible in every dependent interface/check result. Required exact fit cannot be certified from an envelope or an omitted supporting feature. An explicitly allowed reduced-scope check must be labelled with its weakened claim. Existing whole-pair collision exemptions are not evidence that an interface fits.

### 7.2 Direction and frame compatibility (M0 proof, M2 consumption)

Material outward normal, unoriented axis line and chosen mate direction are separate. Add `alignment: aligned | opposed | unoriented` to concentric/revolute constraints, defaulting to `aligned` for legacy calls. An unoriented axis accepts either direction and reports the numerical branch selected from initial placement; it does not establish unique physical rotation. Branch choice remains fixed during one local solve, with no discontinuous sign switch inside a residual evaluation.

For fixed/prismatic frame relationships, add `alignment: aligned | opposed` and `twist_deg` (default 0). Let the A frame axes be X,Y,Z, `s=+1` for aligned and `s=-1` for opposed, and θ be twist about A's +Z. The target B axes are `X'=cosθ X+sinθ Y`, `Z'=s Z`, `Y'=Z'×X'`. This is a proper rotation, including for opposed normals. Fixed also coincides origins; prismatic leaves translation along Z free. Existing plane `opposed` and signed distance retain their meanings. Raw unoriented alignment is invalid for a full fixed frame.

For one anchored and one free rigid body: point coincidence leaves 3 DOF; plane relation leaves 3; concentric leaves 2; revolute leaves 1; prismatic leaves 1; fixed leaves 0, away from degenerate geometry. A direction-only relation is not a coaxial relation. These contracts, not CadQuery operation names, define compatibility. The same direction interpretation is used by solving and checking.

### 7.3 Measured relationship/check contract (M2)

Add `geometric_relationship` checks with explicit `a`/`b` occurrence targets, optional semantic surface regions, required numerical `tolerance`, `max_overlap_mm3`, optional `min_clearance_mm`, and explicit allowed-contact region pairs. Values are finite/nonnegative. A measurement receipt reports `separated | touching | overlapping | contained | indeterminate`, boundary distance in mm, overlap volume in mm³ where requested, closest-point pairs/support refs where available, and actual predicate tolerances.

Boundary distance is not signed penetration depth. Zero distance does not distinguish contact from overlap. Full containment must be detected independently of a positive boundary gap. Minimum clearance requires a separated relationship and adequate distance; allowed face contact cannot exempt interference elsewhere on that part pair. If region-restricted classification or witness computation is unsupported, report that fact; a required claim fails rather than falling back to whole-pair exemption. Numerical uncertainty is `indeterminate`, never pass.

Pose samples are explicit, bounded rigid transforms in assembly space (maximum 32/check initially). Receipts list each checked pose; no continuous-motion clearance claim follows from discrete samples. A failed required check does not replace accepted session source. Existing Collision/Opening/Containment contracts remain compatible and retain their original scope.

## 8. Persistence, budgets and performance

- Immutable query artifacts can be reopened without their generating process. Relocation and digest verification are required. Snapshot formats are versioned; incompatible kernel/index versions fail explicitly. No transparent cache repair changes geometry.
- Optional indices are caches, never authority for exact fit or material checks. Cache keys include snapshot/revision, recognizer version, tolerances, target/space, filters and requested fields. Display camera changes do not invalidate geometry indices; placement changes invalidate occurrence-space results.
- Descriptor queries can reuse verified indices without loading every geometric definition. Evaluation/recognition loads only the required exact definitions. A batch does not copy the B-rep for each instance.
- Initial advertised ceilings: 64 operations/batch, 256 explicit refs/operation, 256 returned items/page, 2 MiB request JSON, 8 MiB inline receipt, 64 MiB total exact query-snapshot payload. Per batch, traverse at most 10,000 unique faces, 50,000 unique edges and 200,000 adjacency incidences; perform at most 20,000 material probes and 10,000 pattern hypotheses. Count repeated-definition topology once but count each actually performed transformed probe/hypothesis. These are proposed ceilings, not measured capacity promises; M0 must test them and explicitly revise the spec if infeasible.
- Existing input/STEP import quotas remain enforced. Query support must not turn external STEP into a trusted generated snapshot merely to bypass import limits. Budget exhaustion returns `QUERY_LIMIT` with partial diagnostics or `RESOURCE_LIMIT`, never an apparently exhaustive empty result.
- Use the SDK's existing process wall limit (default 30 s, configurable up to 120 s). The host controls ceilings; an agent request may lower but not raise them. Geometry/query memory is measured independently of inference, Chromium and video encoding.
- Reference fixtures must fit a **1 vCPU / 512 MiB Linux arm64** sandbox including the CLI/native descendants and charged output/cache pages. On x64, run functional checks and state whether emulation is used; do not treat emulated timing as a native performance comparison.

Benchmarks report five fresh-process runs and five repeated queries after one explicit snapshot preparation. Separate build/snapshot creation, load, indexing, recognition, selection, mate solving and validation; report median, p95 with sample count, maximum observed wall time, peak cgroup memory, artifact sizes and failures. A descriptor query must show zero geometry-build/tessellation/export calls. No fixed speedup is promised before measurement.

Compare CadQuery/OCCT only on the same geometry, precision, scope and outputs. Include both cold-start and loaded-shape query modes, and do not credit Colocad for omitted recognition/validation. For repeated occurrences, verify definitions are reused before measuring. Any proposed stricter latency target must be backed by the M1 baseline and recorded as an explicit acceptance revision.

## 9. Error contract

| Code | Meaning / required behavior |
| --- | --- |
| `QUERY_UNSUPPORTED` | Schema, operation, profile or geometric subtype unsupported |
| `QUERY_SNAPSHOT_REQUIRED` | Artifact lacks exact query snapshot; explicit rebuild required |
| `QUERY_SNAPSHOT_INCOMPATIBLE` | Unsupported serializer/kernel/index version |
| `ARTIFACT_INTEGRITY` | Manifest or exact payload/name binding digest mismatch |
| `QUERY_REVISION_MISMATCH` | Requested revision differs from opened snapshot |
| `TOPOLOGY_REF_INVALID` | Wrong owner/kind/path or missing reference in this snapshot |
| `QUERY_ARGUMENT` | Invalid units, vector, UV, filter, tolerance or mixed target |
| `QUERY_PRECISION_UNSUPPORTED` | Requested precision is unsupported by known input/kernel accuracy |
| `QUERY_LIMIT` / `RESOURCE_LIMIT` | Exploration, output, payload, time or memory budget exceeded |
| `SELECTION_CARDINALITY` | Complete selection violates required count, including unresolved tied rank |
| `DATUM_REFERENCE` | Missing definition/dependency or cyclic datum dependency |
| `DATUM_AMBIGUOUS` | Geometry does not determine a required origin/direction uniquely |
| `DATUM_DEGENERATE` | Zero direction, singular or nonorthogonal frame derivation |
| `DATUM_UNSUPPORTED` | Requested derivation/type is outside the supported geometric domain |
| `INTERFACE_NOT_FOUND` | Required declared interface/supporting geometry absent |
| `INTERFACE_AMBIGUOUS` | Cardinality, origin/orientation or correspondence not resolved |
| `INTERFACE_CHANGED` | Pinned definition or required geometric relationship changed |
| `INTERFACE_UNSUPPORTED` | Required interpretation cannot be established in the supported domain |
| `INTERFACE_INCOMPATIBLE` | Required measured fit/correspondence check fails |

Preserve existing solver error codes. Errors include query/interface/mate identity, affected references, measured deviation where available, and an actionable cause. Unsupported or ambiguous geometry is not cured by taking the nearest face. The viewer's exploded or sectioned display does not affect any exact query.

## 10. Acceptance matrix and TDD

Fixtures below are **planned**. They are not existing test results. Each fixture records generator or source/license/hash, units, expected analytic quantities, thresholds and the independent verifier/version. Imported examples must be independently authored/exported rather than only round-tripping this kernel.

Pin Q01 to a `100 × 60 × 12 mm` block with XY origin at its center and Z in `[0,12]`: a Ø6 through hole at `(-30,-20)`, a Ø10 planar blind hole of depth 8 at `(30,-20)`, and a Ø6 through hole with Ø10 counterbore depth 3 at `(0,15)`. All enter from Z=12 along -Z. Expected total volume is `72000 - 464π mm³`; the top-face material area is `6000 - 59π mm²`. This checks segment depth and material removal, not just a plausible rendering.

Default analytic-fixture acceptance: position/diameter/depth error ≤ `1e-5 mm`, angle ≤ `1e-7 rad`, and area/volume error ≤ `max(1e-6 in the corresponding unit, 1e-6 × |expected|)`. Material witnesses stay at least `10 × linear tolerance` away from boundaries; separate boundary tests must report boundary/uncertain outcomes rather than force an inside/outside result. Imported-product fixtures set source-supported thresholds before running (starting reconstruction threshold: `0.01 mm`, `1e-4 rad`, never an asserted manufacturer tolerance). Changes to thresholds require a recorded reason, not a post-failure adjustment to obtain a pass.

| ID | Fixture/change | Required observable result |
| --- | --- | --- |
| Q01 | Box with offset bore, blind bore and counterbore | Correct plane/cylinder/edge descriptors, sizes, axes and area/volume sanity |
| Q02 | Arbitrary rigid rotation and nested instances | Local descriptors invariant; world points/normals/axes transform correctly; two copies have distinct occurrence refs |
| Q03 | Annular trimmed face and periodic cylinder seam | UV/trim membership and repeated edge uses correct; centroid is not an interior witness; no hole-recognition dependency |
| Q04 | Save/reopen/relocate snapshot; change payload/name map | Same refs resolve on unchanged snapshot; altered data fails integrity |
| Q05 | Parameter edit; named face split/merge/delete | Old refs rejected; semantic resolution returns new refs, ambiguity or missing status; no nearest-face fallback |
| Q06 | Shaft/boss, through hole, planar blind hole, counterbore and split/seam cylinder | Correct material classification, entrances and segment depths; shaft never reported as hole; seam does not double-count a bore |
| Q07 | Intersecting bore, side-open recess, conical tip, spline cylinder | Partial/unsupported result where necessary; no false complete recognition |
| Q08 | Rectangular/circular/linear patterns plus extra holes | Correct pitches/PCD/members; alternative subsets and symmetry retained |
| Q09 | Two coincident-looking faces and incomplete pages | Required uniqueness fails; stable complete pagination; cursor from another revision rejected |
| Q10 | Native declared interface versus imported STEP recognition | Same supported geometric facts within thresholds; provenance remains different |
| A01 | Independently authored compatible motor flange + support STEP | Query → interface selection → mate solve → exact checks; workflow contains no face indices or part-specific placement coordinates |
| A02 | Replace motor with same-interface different envelope; rotate import | Same workflow; correct attachment and explicit clearance checks; reusable definition/occurrence distinction |
| A03 | Wrong pitch/register, missing hole or opposing-normal error | Measured incompatibility; no successful accepted assembly |
| A04 | Symmetric flange without a key; plane+axis only | Discrete alternatives and/or free rotation exposed; no unique fixed-pose claim |
| A05 | Revolute/prismatic attachment and contradictory constraints | Expected DOF/residuals, conflict diagnostics and no silent fixing |
| A06 | KK60 + externally authored mounting bracket at selected travel poses | Source-backed mounting facts, named interface use, exact requested contact/clearance checks; untested pairs stated |
| I01 | SDK, CLI, agent tool, session reopen | Same query semantics, immutable source, stored receipt and failure recovery |
| I02 | Normal query/filter/profile result in viewer | Selected face/axis/group visibly matches native references; display explosion does not change query coordinates |
| P01 | Plate, imported flange, KK60, repeated definitions | Resource and stage measurements per §8; no rebuild for descriptor-only queries |
| R01 | Existing native/Node/integration/viewer tests | Existing APIs/fixtures retain semantics; no forced query overhead on legacy builds |
| S01 | Selected plane → inner loops → circle edges → centers | One bounded native expression; no edges from other planes; explicit ties/cardinality |
| S02 | Rotated imported part with an asymmetric geometric key | Relative recipe follows geometry; symmetric/no-key variant remains ambiguous |
| D01 | Point, plane, cylinder axis, circle center, direction and frame | Correct primitive datums without a hole-group profile; singular/degenerate derivations rejected |
| D02 | Opposed surface frames, aligned/opposed/unoriented axis policies, frame twist | Valid mounting relation succeeds; legacy defaults unchanged; actual conflict fails |
| D03 | Point/plane/concentric/revolute/prismatic/fixed | Required DOF and physical residuals; no artificial constraint from basis completion |
| D04 | Rebuild named datum after dimension/source changes | New refs, semantic re-resolution, explicit missing/ambiguous outcome; no persisted arena index |
| C01 | Touching, 0.2 mm gap, penetration and full containment | Correct distinct relationships, measured distance/volume and unsupported/uncertain states |
| C02 | Allowed installation-face contact plus collision elsewhere; travel samples | Region exception does not hide other collision; receipt lists exactly tested poses |

For each milestone: write failing tests first, retain the initial failure command, implement the contract, run the affected tests, then update the implementation/evidence table. Analytic fixtures give known answers; independent OCCT readback/material/transform checks catch agreement between two implementations of the same bug. Match geometry/adjacency and measurements rather than assuming identical face counts across exporters.

Automated tool sequences establish API/geometry behavior. A live agent run is recorded separately with model/version, source pack, tools, budget, transcript and outcome. Do not label deterministic scripted tests as agent success rates.

## 11. Delivery plan and PR boundaries

| Milestone | Deliverable and gate |
| --- | --- |
| **M0 — feasibility and direction contract** | Audit pinned evaluation/material/serialization support; prove exact snapshot/name/ID reload; reproduce then repair D02 direction conflict with legacy controls; validate budgets. Freeze only tested layer schemas. |
| **M1 — composable geometry and datums** | Immutable snapshots, scoped refs, bounded selection/traversal, descriptors/evaluation, datum derivations and SDK/CLI/tool. Q01–Q05, Q09, S01–S02, D01/D04, I01 and baseline P01. Include minimal external STEP import→query. |
| **M2 — basic assembly and measurements** | Datums directly consumed by existing mates; direction/frame adapter, physical residuals/DOF and explicit measurement/contact checks. D02–D03, C01–C02, A05; no dependency on engineering hole-group recognition. |
| **M3 — engineering interfaces and replacement** | Bounded holes/groups, declared/recognized profiles, interface compatibility and product replacement fixtures. Q06–Q08/Q10, A01–A04/A06; no part-specific workflow recognizers. |
| **M4 — review, docs and release** | Viewer reference highlight, examples, skill/tool guidance, full evidence and resource report. I02 plus complete matrix. Bump/release only after the documented gate is met. |

Each PR includes a small reproducible fixture and stated supported geometry. Kernel fixes are separate from public API adapters where possible; adding a plane query must not require merging a complete product workflow. New constraints reuse the current solver instead of replacing it as a side effect.

Proposed implementation locations: `crates/agent-cad/src/query/{snapshot,topology,descriptors,recognition,interfaces}.rs`, modifications to `model.rs`, `mates.rs`, `assembly.rs`, SDK `query.ts`/generated types, CLI dispatch, session receipt handling and viewer selection overlays. Exact file splitting may change without changing contracts. The authoritative runtime schemas are generated from Rust; do not hand-maintain competing JS/Python schemas.

## 12. Definition of done and current evidence

All matrix rows must have executable evidence or a reviewed explicit scope revision. Public capabilities list supported operations, recognition domain, profile/schema versions and budgets. Examples show declaration and imported recognition, a successful replacement, ambiguity, incompatibility and an edit invalidating bindings. Documentation distinguishes candidate recognition from verified engineering fit and preserves the low-cost local execution model.

**Current implementation evidence:** see the [experiment ledger](experiments/0004/README.md). This revision incorporates all six review findings and defines 28 acceptance rows. Passing one native experiment does not complete a milestone; imported-interface acceptance and performance results require their own recorded evidence.

Primary references: [SolidCore](https://github.com/tangency-labs/solidcore), [USD BRep](https://github.com/NVIDIA-Omniverse/usd-brep), [CadQuery selectors](https://cadquery.readthedocs.io/en/latest/selectors.html), [CadQuery assemblies](https://cadquery.readthedocs.io/en/latest/assy.html). These inform the design; the pinned dependency and passing local tests determine actual capabilities.


## Implementation note — G1 iteration 3, 2026-11-09

The public source implements the supported query/selection/datum/hole/group/interface/relationship contracts, native-ref viewer overlays and SDK/CLI/tools. Relative plane selection includes `planar_coplanar_with: {datum}` in addition to normal-direction predicates; it requires an earlier plane datum and checks offset as well as angle. Bounds are computed from trimmed definition geometry and explicitly labeled conservative transformed AABBs in assembly space; no tight world-box claim is made.

Hole completeness is scoped to the requested support adjacency; uncertainty outside it remains in diagnostics. Full-body queries retain partial status. Linear requested-count subsets, extra holes and symmetry remain visible. Allowed-contact certification is initially supported for opposing planar faces on polyhedral closed solids only. Unsupported region-restricted or curved-contact required claims fail; whole-part distance/overlap measurements are separate. Measurement result status is complete/partial, with a standard paged-result envelope containing one item and an equivalent `measurement` alias.

[Guide](../docs/engineering-queries.md), [foreign/whole-product examples](../examples/engineering-interfaces/README.md) and [ledger](experiments/0004/README.md) state exact acceptance evidence. Lifecycle/resource/parser stress belongs to G3; broad corpus reliability belongs to G2. Packaged alpha.6 binaries do not contain this development implementation.

## Acceptance update — G2/G3, 2026-11-09

The [G2/G3 record](experiments/0005/README.md) closes the declared reliability and engineering-usability gates: 24/24 frozen cases with independent OCCT readback, semantic reference lifecycle and source replacement, real interruption/recovery, execution-policy cache invalidation, bounded parser probes, and two live Pi tasks. The agent/harness/SDK/native run together within 1 vCPU / 512 MiB; the 82-solid product task peaks at 370.57 MiB without OOM. Final regression: 127 Rust, 41 Node on each tested platform and six scripted integration tests. Supported recognition/contact boundaries above remain explicit. G4 and npm publication have not been performed in this acceptance round.
