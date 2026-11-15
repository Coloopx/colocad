//! [INPUT]: Rust公共资源合同、准确UTF-8字节与纯Schema模型。
//! [OUTPUT]: 默认/宿主有限预算、单源/聚合、CLI解析与准确下界回执回归。
//! [POS]: 字节合同不依赖几何容差或源文件路径。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{resources, validate_with_resource_limits, Model, ResourceLimits};
use serde_json::json;
use sha2::{Digest, Sha256};

fn model(texts: &[&str]) -> Model {
    let assets = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            (
                format!("asset{i}"),
                json!({"step":text,"sha256":format!("{:x}",Sha256::digest(text.as_bytes()))}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::from_value(
        json!({"version":1,"units":"mm","parameters":{},"assets":assets,
        "features":[{"id":"box","op":"box","origin":[0,0,0],"size":[1,1,1]}],"output":"box"}),
    )
    .unwrap()
}

#[test]
fn utf8_single_and_aggregate_boundaries_are_independent() {
    let limits = ResourceLimits {
        input_bytes: 1024,
        step_bytes: 6,
        total_step_bytes: 12,
    };
    validate_with_resource_limits(&model(&["日日", "日日"]), &limits).unwrap();
    let error = validate_with_resource_limits(&model(&["日日x"]), &limits).unwrap_err();
    assert_eq!(error.code, "RESOURCE_LIMIT");
    assert_eq!(error.details.as_ref().unwrap()["resource"], "stepBytes");
    assert_eq!(error.details.as_ref().unwrap()["observed_bytes"], 7);
    let limits = ResourceLimits {
        step_bytes: 8,
        ..limits
    };
    let error = validate_with_resource_limits(&model(&["日日x", "日日"]), &limits).unwrap_err();
    assert_eq!(
        error.details.as_ref().unwrap()["resource"],
        "totalStepBytes"
    );
    assert_eq!(error.details.as_ref().unwrap()["observed_bytes"], 13);
}

#[test]
fn defaults_and_hard_ceilings_cannot_be_overridden_by_model_data() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.input_bytes, 32 * 1024 * 1024);
    assert_eq!(limits.step_bytes, 24 * 1024 * 1024);
    assert_eq!(limits.total_step_bytes, 24 * 1024 * 1024);
    for value in [0, ResourceLimits::CEILINGS.step_bytes + 1] {
        assert!(ResourceLimits {
            step_bytes: value,
            ..limits
        }
        .validate()
        .is_err());
    }
    assert!(ResourceLimits {
        total_step_bytes: limits.input_bytes + 1,
        ..limits
    }
    .validate()
    .is_err());
    let mut raw = serde_json::to_value(model(&[])).unwrap();
    raw["resourceLimits"] = json!({"stepBytes":999999999});
    assert!(serde_json::from_value::<Model>(raw).is_err());
}

#[test]
fn cli_options_are_host_only_bounded_and_consumed_once() {
    let mut args = vec![
        "inspect-step".to_string(),
        "--resource-limits".into(),
        "{\"inputBytes\":67108864}".into(),
    ];
    let limits = resources::take_host_limits(&mut args).unwrap();
    assert_eq!(limits.input_bytes, 64 * 1024 * 1024);
    assert_eq!(args, ["inspect-step"]);
    for raw in [
        "{\"stepBytes\":0}",
        "{\"unknown\":1}",
        "{\"inputBytes\":1.5}",
    ] {
        let mut args = vec!["validate".into(), "--resource-limits".into(), raw.into()];
        assert_eq!(
            resources::take_host_limits(&mut args).unwrap_err().code,
            "INVALID_RESOURCE_LIMITS"
        );
    }
    let mut args = vec![
        "validate".into(),
        "--resource-limits".into(),
        "{}".into(),
        "--resource-limits".into(),
        "{}".into(),
    ];
    assert!(resources::take_host_limits(&mut args).is_err());
}

#[test]
fn truncated_oversize_stdin_reports_a_lower_bound_instead_of_a_fictional_full_size() {
    resources::check_bytes("inputBytes", 12, 12, false).unwrap();
    let error = resources::check_bytes("inputBytes", 13, 12, true).unwrap_err();
    let details = error.details.unwrap();
    assert_eq!(details["observed_bytes"], 13);
    assert_eq!(details["limit_bytes"], 12);
    assert_eq!(details["observed_is_lower_bound"], true);
}
