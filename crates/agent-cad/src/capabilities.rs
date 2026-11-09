//! Kernel policy and advertised implementation capabilities.
use crate::*;
pub(crate) fn kernel_policy() -> Value {
    json!({"name":"solidcore","revision":KERNEL_REV,"tolerances":"kernel_defaults","upstream_fixes":["transform_scales_each_tolerance","transform_model_size_from_body_box","fillet_chamfer_cone_on_arc","shell_edges_on_analytics_exact"],"scoped_fixes":{"thicken":["shell_one_construction"],"measurement":["precise_properties_body_length_scale","precise_properties_split_at_surface_knots","precise_properties_span_end_from_below","precise_properties_trim_base_v","integrator_propagates_failure"]},"volume_integration_accuracy":1e-8})
}
pub fn capabilities() -> Value {
    let schema = serde_json::to_value(schemars::schema_for!(Feature)).expect("serializable schema");
    let operations = schema["oneOf"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["properties"]["op"]["const"].as_str())
        .collect::<Vec<_>>();
    json!({"protocol":"agent-cad/1","executor_version":EXECUTOR_VERSION,"execution_policy":EXECUTION_POLICY,"kernel":kernel_policy(),"units":["mm"],
        "operations":operations,
        "profiles":["rectangle","circle","ellipse","polygon","outline_lines_and_arcs","constrained_sketch"],
        "assembly":{"named_instances":true,"hierarchy":true,"shared_definitions":true,"construction_volumes":true,"representation_declarations":true,"poses":["normal","internal","exploded"],"checks":["collision","containment","opening","interface_compatibility","geometric_relationship"],"step":"native_product_instance_tree","mates":["fixed","coincident","concentric","plane","distance","revolute","prismatic"],"mate_direction":{"axis":["aligned","opposed","unoriented"],"frame":["aligned","opposed"],"frame_twist_deg":true,"default":"aligned","unoriented_branch":"initial_pose_fixed_during_local_solve"},"mate_physical_residual_units":["mm","rad"]},
        "exports":["step_exact_solids","step_exact_surfaces","feature_ir","png","glb","stl"],
        "query":{"schema":"colocad.query/1","snapshot":"colocad.query-manifest/1","operations":["faces","edges","vertices","loops","describe","select","datums","named_datums","evaluate_face","holes","hole_groups","interfaces","measure"],"selection":["entities","refs","filter","children","ancestors","siblings","union","intersection","difference","rank","expect"],"datum_derivations":["vertex_or_circle_point","planar_face","cylinder_or_circle_axis","straight_edge_direction","declared_design_point","ordered_point_difference","right_handed_frame"],"persistent_datums":true,"relative_predicates":["planar_normal_parallel_to","planar_normal_aligned_with","planar_coplanar_with"],"hole_domain":"connected_analytic_cylinders_planar_ends","hole_patterns":["linear","rectangular","circular","unstructured"],"interface_profiles":["planar_hole_mount/v1"],"contact_exception_domain":"certified_polyhedral_separating_plane","native_ref_display":"build_time_outline_artifact","limits":{"request_bytes":2*1024*1024,"receipt_bytes":8*1024*1024,"snapshot_bytes":64*1024*1024,"batch":64,"page":256,"expression_depth":8,"expression_nodes":128,"adjacency_visits":200_000},"restrictions":["bounds_are_transformed_definition_aabbs","complex_bores_return_partial","nonseparating_or_curved_contact_regions_unsupported","source_accuracy_unknown"]},
        "naming":{"source":"native_topology_notifications","split_merge":true,"parameter_replay":true,"ambiguity":"expected_count_required","persistence":"feature_IR_names; not foreign STEP entity numbers"},
        "sketch":{"solver":"bounded_local_levenberg_marquardt","degrees_of_freedom":true,"constraint_residuals":true},
        "surfaces":{"gordon_network":{"curve_family_size":[2,8],"bezier_degrees":[1,7],"intersection_tolerance_mm":1e-7},"bezier_control_edit":true,"c2_boundary_jet_bridge":true,"sew":true,"thicken":true,"class_a_certified":false},
        "preview":{"instance_mesh_reuse":true,"revision_mesh_reuse":true,"halfspace_section":true,"analytic_full_cylinder_and_cone":true},"incremental":{"dependency_keys":true,"exact_checkpoints":true,"replay_for_named_or_region_features":true},"inspection_regions":{"automatic_propagation":true,"computed_on_demand":true,"exact_required_option":true},
        "limits":{"input_bytes":MAX_INPUT_BYTES,"features":MAX_FEATURES,"cutters":MAX_CUTTERS,"step_asset_bytes":24*1024*1024,"step_entities":100_000,"generated_step_entities_per_definition":1_000_000,"instances":128,"groups":512,"checked_pairs":8128,"solver_variables":128,"preview_triangles":200_000},
        "restrictions":{"profile_loft":"2..12 corresponding-curve sections; degree 1..3","sweep":"continuous planar path","curve_sweep":"one line/arc/Bezier path; explicit profile at path start","offset_profile":"must retain one connected boundary","step_import":"rigid product and mapped-item placements; shared definitions; mixed representation units and nonrigid mappings refused","surface_bridge":"Bezier U-end to U-start; matching V degree; analytic C2 jets; no global Class-A certification","solvers":"local branch and local Jacobian rank; not a proof of global uniqueness","fillet_shell":"kernel failures surfaced; optional explicit on_fail record preserves the input with a degradation receipt","large_grid":"1024 holes supported; may require host wallMs=120000"},
        "unsupported":["general_class_a_certification","arbitrary_multi_rail_surface_network","commercial_native_feature_tree"],
        "execution":"Agent, Harness and the CAD package run in the same sandbox; Harness invokes local module actions"})
}
