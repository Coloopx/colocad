//! Trim classification adapter for the pinned kernel. Not yet a CLI query API.
use crate::*;
use solidcore::intersect::point_class::PointClassType;
use solidcore::query::api_queries::face_classify_uv;
use solidcore::{FaceId, Point2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrimStatus {
    Inside,
    Boundary,
    OutsideTrim,
}

pub fn classify_uv(k: &Kernel, b: &mut Brep, face: FaceId, uv: Point2) -> Result<TrimStatus> {
    if !uv.x.is_finite() || !uv.y.is_finite() {
        return Err(CadError::new(
            "QUERY_ARGUMENT",
            "UV coordinates must be finite",
        ));
    }
    match face_classify_uv(k, b, face, uv)? {
        PointClassType::Face => Ok(TrimStatus::Inside),
        PointClassType::Edge | PointClassType::Vertex => Ok(TrimStatus::Boundary),
        // This mapping is specific to the successful face classifier, whose
        // documented outside result is Unknown (query/face/p2.rs). Do not map
        // arbitrary Unknown classifications or kernel errors to outside_trim.
        PointClassType::Unknown => Ok(TrimStatus::OutsideTrim),
        class => Err(CadError::new(
            "QUERY_UNSUPPORTED",
            format!("unexpected face classification: {class:?}"),
        )),
    }
}
