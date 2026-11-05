use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct CadError {
    pub code: &'static str,
    pub message: String,
    pub feature_id: Option<String>,
}
impl CadError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            feature_id: None,
        }
    }
}
impl std::fmt::Display for CadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for CadError {}
impl From<solidcore::SolidError> for CadError {
    fn from(e: solidcore::SolidError) -> Self {
        Self::new("KERNEL_ERROR", e.to_string())
    }
}
