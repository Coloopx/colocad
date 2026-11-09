export type CurveClass = "line" | "circle" | "ellipse" | "nurbs" | "other";
export type DatumInput = {
    "datum": string;
};
export type Derive = ({
    "kind": "point";
}) | ({
    "kind": "plane";
}) | ({
    "kind": "axis";
}) | ({
    "kind": "direction";
});
export type DirectionInput = ([number, number, number]) | ({
    "datum": string;
});
export type Filter = {
    "all"?: Array<Filter>;
    "any"?: Array<Filter>;
    "area_mm2"?: (Range) | (null);
    "curve_type"?: (CurveClass) | (null);
    "length_mm"?: (Range) | (null);
    "loop_role"?: (LoopRole) | (null);
    "name"?: string | null;
    "not"?: (Filter) | (null);
    "planar_coplanar_with"?: (DatumInput) | (null);
    "planar_normal_aligned_with"?: (DirectionInput) | (null);
    "planar_normal_parallel_to"?: (DirectionInput) | (null);
    "radius_mm"?: (Range) | (null);
    "surface_type"?: (SurfaceClass) | (null);
};
export type HoleFilter = {
    "axis_parallel_to"?: (DirectionInput) | (null);
    "diameter_mm"?: (Range) | (null);
    "opening_on"?: Array<TopologyRef>;
    "termination"?: (Termination) | (null);
};
export type HoleGroupFilter = {
    "equal_diameter"?: boolean;
    "member_count"?: number | null;
    "pattern"?: (HolePattern) | (null);
};
export type HolePattern = "linear" | "rectangular" | "circular" | "unstructured";
export type Kind = "face" | "edge" | "vertex" | "loop";
export type LoopRole = "outer" | "inner" | "unknown";
export type Query = ({
    "cursor"?: string | null;
    "id": string;
    "limit"?: number;
}) & (({
    "a": Target;
    "b": Target;
    "op": "measure";
}) | ({
    "op": "faces";
    "where"?: Filter;
}) | ({
    "op": "edges";
    "where"?: Filter;
}) | ({
    "op": "vertices";
    "where"?: Filter;
}) | ({
    "op": "loops";
    "where"?: Filter;
}) | ({
    "op": "select";
    "selection": Selection;
}) | ({
    "op": "describe";
    "refs": Array<TopologyRef>;
}) | ({
    "derive": Derive;
    "op": "datums";
    "selection": Selection;
}) | ({
    "ids"?: Array<string> | null;
    "op": "interfaces";
    "profile"?: string | null;
    "support_faces"?: Array<TopologyRef>;
    "where"?: HoleGroupFilter;
}) | ({
    "op": "hole_groups";
    "support_faces"?: Array<TopologyRef>;
    "where"?: HoleGroupFilter;
}) | ({
    "op": "holes";
    "where"?: HoleFilter;
}) | ({
    "ids": Array<string>;
    "op": "named_datums";
}) | ({
    "op": "evaluate_face";
    "ref": TopologyRef;
    "uv": [number, number];
}));
export type Range = {
    "max"?: number | null;
    "min"?: number | null;
};
export type RankBy = ({
    "property": "area_mm2";
}) | ({
    "property": "length_mm";
}) | ({
    "direction": DirectionInput;
    "property": "center_projection";
});
export type Selection = ({
    "entity": Kind;
    "kind": "entities";
}) | ({
    "kind": "refs";
    "refs": Array<TopologyRef>;
}) | ({
    "input": Selection;
    "kind": "filter";
    "where": Filter;
}) | ({
    "entity": Kind;
    "input": Selection;
    "kind": "children";
}) | ({
    "entity": Kind;
    "input": Selection;
    "kind": "ancestors";
}) | ({
    "entity": Kind;
    "input": Selection;
    "kind": "siblings";
    "via": Kind;
}) | ({
    "inputs": Array<Selection>;
    "kind": "union";
}) | ({
    "inputs": Array<Selection>;
    "kind": "intersection";
}) | ({
    "kind": "difference";
    "left": Selection;
    "right": Selection;
}) | ({
    "by": RankBy;
    "descending"?: boolean;
    "input": Selection;
    "kind": "rank";
    "rank": number;
    "tie_tolerance": number;
}) | ({
    "count": number;
    "input": Selection;
    "kind": "expect";
});
export type Space = "definition" | "assembly";
export type SurfaceClass = "plane" | "cylinder" | "cone" | "sphere" | "torus" | "nurbs" | "other";
export type Target = ({
    "definition_id": string;
}) | ({
    "instance_path": Array<string>;
});
export type Termination = "through" | "blind" | "unknown";
export type Tolerance = {
    "angular_rad": number;
    "linear_mm": number;
};
export type TopologyRef = {
    "definition_id": string;
    "definition_sha256": string;
    "instance_path"?: Array<string> | null;
    "kind": Kind;
    "query_revision_id": string;
    "topology_id": string;
};
export type Request = {
    "queries": Array<Query>;
    "query_revision_id": string;
    "schema": string;
    "space": Space;
    "target": Target;
    "tolerance": Tolerance;
};
