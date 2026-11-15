/**
 * [INPUT]: Host-only STEP count policy, external Part21 input and downstream capacity counts.
 * [OUTPUT]: Validated scoped limits, structured resource errors and one external parser identity.
 * [POS]: Native host boundary; agent Model JSON cannot grant itself a larger budget.
 * [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
 */
use crate::*;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct StepImportLimits {
    pub max_entities: usize,
    pub max_definitions: usize,
    pub max_occurrences: usize,
    pub max_groups: usize,
}
impl Default for StepImportLimits {
    fn default() -> Self {
        Self {
            max_entities: 100_000,
            max_definitions: 128,
            max_occurrences: 128,
            max_groups: 512,
        }
    }
}
impl StepImportLimits {
    pub const HARD: Self = Self {
        max_entities: 2_000_000,
        max_definitions: 1024,
        max_occurrences: 4096,
        max_groups: 8192,
    };
    pub fn validate(self) -> Result<()> {
        for (name, actual, ceiling) in [
            ("maxEntities", self.max_entities, Self::HARD.max_entities),
            (
                "maxDefinitions",
                self.max_definitions,
                Self::HARD.max_definitions,
            ),
            (
                "maxOccurrences",
                self.max_occurrences,
                Self::HARD.max_occurrences,
            ),
            ("maxGroups", self.max_groups, Self::HARD.max_groups),
        ] {
            if actual == 0 || actual > ceiling {
                return Err(resource(name, actual, ceiling, "host_configuration"));
            }
        }
        Ok(())
    }
    pub(crate) fn parser(self) -> step_import::Limits {
        step_import::Limits {
            max_entities: self.max_entities,
            max_list_depth: 24,
            max_ref_depth: 32,
            max_list_len: 16_384,
        }
    }
}
thread_local! { static ACTIVE: Cell<StepImportLimits> = Cell::new(StepImportLimits::default()); }
pub fn current() -> StepImportLimits {
    ACTIVE.with(Cell::get)
}
struct Reset(StepImportLimits);
impl Drop for Reset {
    fn drop(&mut self) {
        ACTIVE.with(|v| v.set(self.0));
    }
}
/// Synchronous, thread-local host scope; restored on success, error and unwind.
pub fn with_import_limits<T>(
    limits: StepImportLimits,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    limits.validate()?;
    let _reset = Reset(ACTIVE.with(|v| v.replace(limits)));
    operation()
}
pub fn resource(name: &str, actual: usize, allowed: usize, stage: &str) -> CadError {
    let mut error = CadError::new(
        "RESOURCE_LIMIT",
        format!("{name}: {actual} exceeds allowed {allowed} ({stage})"),
    );
    error.details = Some(
        json!({"resource_limit":{"resource":name,"actual":actual,"allowed":allowed,"stage":stage}}),
    );
    error
}
pub(crate) fn check(name: &str, actual: usize, allowed: usize, stage: &str) -> Result<()> {
    if actual > allowed {
        return Err(resource(name, actual, allowed, stage));
    }
    Ok(())
}
fn parse_error(error: step_import::ParseError) -> CadError {
    let allowed = current().max_entities;
    if error.message == format!("more than {allowed} entities") {
        let mut out = resource("step_entities", allowed + 1, allowed, "external_parse");
        out.details.as_mut().unwrap()["resource_limit"]["actual_is_lower_bound"] = json!(true);
        out.details.as_mut().unwrap()["line"] = json!(error.line);
        return out;
    }
    CadError::new("STEP_ASSEMBLY", error.to_string())
}
pub fn parse_external(text: &str) -> Result<step_import::part21::StepFile> {
    step_import::part21::parse(text.as_bytes(), &current().parser()).map_err(parse_error)
}
pub(crate) fn import_external(text: &str) -> Result<step_import::model::Model> {
    let model = step_import::import(text.as_bytes(), &current().parser()).map_err(parse_error)?;
    check(
        "step_definitions",
        model.bodies.len(),
        current().max_definitions,
        "external_map",
    )?;
    Ok(model)
}
/// Remove exactly one host flag before native command validation; unknown JSON keys are rejected.
pub fn host_args(args: &mut Vec<String>) -> Result<StepImportLimits> {
    let Some(at) = args.iter().position(|a| a == "--step-import-limits") else {
        return Ok(Default::default());
    };
    if at + 1 == args.len()
        || args
            .iter()
            .skip(at + 1)
            .any(|a| a == "--step-import-limits")
    {
        return Err(CadError::new(
            "USAGE",
            "one --step-import-limits JSON value required",
        ));
    }
    let limits: StepImportLimits = serde_json::from_str(&args[at + 1])
        .map_err(|e| CadError::new("STEP_IMPORT_LIMITS_INVALID", e.to_string()))?;
    args.drain(at..=at + 1);
    limits.validate()?;
    Ok(limits)
}
pub(crate) fn feature_capacity(model: &Model) -> usize {
    if model.features.iter().all(|f| {
        matches!(
            f,
            Feature::ImportStep {
                definition: true,
                ..
            }
        )
    }) {
        current().max_definitions
    } else {
        MAX_FEATURES
    }
}
pub(crate) fn downstream(model: &Model, queryable: bool) -> Result<()> {
    if queryable || model.preview.is_some() {
        let prefix = if queryable { "query" } else { "preview" };
        check(
            &format!("{prefix}_occurrences"),
            model.assembly.len(),
            128,
            "downstream_capacity",
        )?;
        check(
            &format!("{prefix}_definitions"),
            model.features.len(),
            128,
            "downstream_capacity",
        )?;
    }
    Ok(())
}

/// Build under a host count policy without changing the declarative model.
pub fn build_revision_with_import_limits(
    model: &Model,
    cache: &mut incremental::Checkpoints,
    queryable: Option<bool>,
    limits: StepImportLimits,
) -> Result<Build> {
    with_import_limits(limits, || {
        crate::build_revision_with_options(model, cache, queryable)
    })
}
pub fn validate_with_import_limits(model: &Model, limits: StepImportLimits) -> Result<()> {
    with_import_limits(limits, || crate::validate(model))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_scope_is_nested_and_restored_after_error_and_unwind() {
        let outer = StepImportLimits {
            max_entities: 7,
            ..Default::default()
        };
        with_import_limits(outer, || {
            with_import_limits(
                StepImportLimits {
                    max_entities: 6,
                    ..outer
                },
                || {
                    assert_eq!(current().max_entities, 6);
                    Ok(())
                },
            )?;
            assert_eq!(current(), outer);
            assert!(with_import_limits(
                StepImportLimits {
                    max_entities: 0,
                    ..outer
                },
                || Ok(())
            )
            .is_err());
            assert_eq!(current(), outer);
            Ok(())
        })
        .unwrap();
        let caught = std::panic::catch_unwind(|| {
            with_import_limits(outer, || -> Result<()> { panic!("scope test") })
        });
        assert!(caught.is_err());
        assert_eq!(current(), StepImportLimits::default());
    }
    #[test]
    fn host_flags_reject_unknown_and_duplicate_policy_without_mutating_model() {
        let mut args = vec![
            "inspect-step".into(),
            "--step-import-limits".into(),
            r#"{"maxOccurrences":129}"#.into(),
        ];
        assert_eq!(host_args(&mut args).unwrap().max_occurrences, 129);
        assert_eq!(args, vec!["inspect-step"]);
        for text in [
            r#"{"maxOccurrences":0}"#,
            r#"{"maxOccurrences":4097}"#,
            r#"{"maxEntities":1.5}"#,
            r#"{"surprise":1}"#,
        ] {
            let mut args = vec![
                "inspect-step".into(),
                "--step-import-limits".into(),
                text.into(),
            ];
            assert!(host_args(&mut args).is_err());
        }
    }
    #[test]
    fn external_points_at_n_and_n_plus_one_do_not_use_generated_budget() {
        let text = |n| {
            format!(
                "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
                (1..=n)
                    .map(|id| format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));\n"))
                    .collect::<String>()
            )
        };
        with_import_limits(StepImportLimits { max_entities: 3, ..Default::default() }, || {
            assert_eq!(parse_external(&text(3))?.entities.len(), 3);
            let error = parse_external(&text(4)).unwrap_err();
            assert_eq!(error.code,"RESOURCE_LIMIT");
            assert_eq!(error.details.unwrap()["resource_limit"],json!({"resource":"step_entities","actual":4,"allowed":3,"stage":"external_parse","actual_is_lower_bound":true}));
            Ok(())
        }).unwrap();
        assert_eq!(current(), StepImportLimits::default());
    }
}
