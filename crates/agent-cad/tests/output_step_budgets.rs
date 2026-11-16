//! [INPUT]: 公开小盒体、现有Rust/CLI入口与JSON宿主预算。
//! [OUTPUT]: 输出预算单体/共享装配边界、缓存降额、能力一致及IR拒绝回归。
//! [POS]: PR6最小输出预算回归；Windows待编译执行，不继承组合版本接受。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build, build_revision_with_resource_limits, incremental::Checkpoints, Model, ResourceLimits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::{fs, path::PathBuf};
use std::process::{Command, Stdio};

const MIB: usize = 1024 * 1024;

struct Revision(PathBuf);
impl Revision {
    fn save(assembly: bool, built: &agent_cad::Build) -> Self {
        let path = std::env::temp_dir().join(format!("colocad-output-cache-{}-{assembly}", std::process::id()));
        fs::create_dir(&path).unwrap();
        let revision = Self(path);
        let mut report = built.report.clone();
        report["artifact_sha256"] = json!({});
        for (name, bytes) in &built.artifacts {
            fs::write(revision.0.join(name), bytes).unwrap();
            report["artifact_sha256"][name] = json!(format!("{:x}", Sha256::digest(bytes)));
        }
        fs::write(revision.0.join("report.json"), serde_json::to_vec(&report).unwrap()).unwrap();
        revision
    }
}
impl Drop for Revision { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

fn model(assembly: bool) -> Model {
    let mut raw: Value = serde_json::from_str(include_str!("fixtures/output-budget-box.json")).unwrap();
    if assembly {
        raw.as_object_mut().unwrap().remove("output");
        raw["assembly"] = json!([
            {"id":"first","input":"box","translation":[0,0,0]},
            {"id":"second","input":"box","translation":[10,0,0]}
        ]);
    }
    serde_json::from_value(raw).unwrap()
}

fn policy(bytes: usize) -> ResourceLimits {
    serde_json::from_value(json!({"outputStepBytes":bytes}))
        .expect("host resourceLimits must accept the independent outputStepBytes option")
}

fn reject(error: agent_cad::CadError, limit: usize) {
    reject_json(&serde_json::to_value(error).unwrap(), limit);
}

fn reject_json(error: &Value, limit: usize) {
    assert_eq!(error["code"], "RESOURCE_LIMIT");
    let details = &error["details"];
    assert_eq!(details["resource"], "outputStepBytes");
    assert_eq!(details["limit_bytes"], limit);
    assert!(details["observed_bytes"].as_u64().unwrap() > limit as u64);
    assert!(details["observed_is_lower_bound"].is_boolean());
}

fn boundary(assembly: bool) {
    let source = model(assembly);
    let default = build(&source).unwrap();
    assert_eq!(default.report["validation"]["valid"], true);
    assert_eq!(default.report["geometry"]["solids"], if assembly {2} else {1});
    let bytes = default.step.len();
    assert!(bytes > 1024 && bytes < MIB, "fixture must remain tiny");
    let mut cache = Checkpoints::revision(None).unwrap();
    let accepted = build_revision_with_resource_limits(&source, &mut cache, None, &policy(bytes)).unwrap();
    assert_eq!(accepted.step.len(), bytes);
    assert_eq!(accepted.report["resource_limits"]["outputStepBytes"], bytes);
    assert_eq!(accepted.report["resource_usage"]["output_step_bytes"], bytes);
    assert_eq!(accepted.report["resource_usage"]["total_step_bytes"], 0);
    let revision = Revision::save(assembly, &accepted);
    let mut strict_cache = Checkpoints::revision(Some(&revision.0)).unwrap();
    let error = build_revision_with_resource_limits(&source, &mut strict_cache, None, &policy(bytes - 1))
        .err().expect("current stricter output policy must also reject restored checkpoints");
    reject(error, bytes - 1);
    let mut cache = Checkpoints::revision(Some(&revision.0)).unwrap();
    let restored = build_revision_with_resource_limits(&source, &mut cache, None, &policy(bytes + 1)).unwrap();
    assert_eq!(restored.report["validation"]["valid"], true);
    assert_eq!(restored.report["features"][0]["cache_hit"], true, "must actually restore native checkpoint");
    assert_eq!(restored.step.len(), bytes);
    if assembly {
        assert_eq!(restored.step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
        assert_eq!(restored.report["assembly"]["instances"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn single_body_output_boundary_and_cached_replay() { boundary(false); }

#[test]
fn shared_assembly_output_boundary_and_cached_replay() { boundary(true); }

#[test]
fn output_defaults_ceiling_and_input_budgets_are_independent() {
    let defaults = serde_json::to_value(ResourceLimits::default()).unwrap();
    assert_eq!(defaults["outputStepBytes"], 32 * MIB);
    assert_eq!(defaults["inputBytes"], 32 * MIB);
    assert_eq!(defaults["stepBytes"], 24 * MIB);
    assert_eq!(defaults["totalStepBytes"], 24 * MIB);
    assert_eq!(serde_json::to_value(ResourceLimits::CEILINGS).unwrap()["outputStepBytes"], 512 * MIB);
    // No input <= output relation: this is a generated artifact, not an imported asset.
    let raised = policy(64 * MIB);
    raised.validate().unwrap();
    let raised = serde_json::to_value(raised).unwrap();
    for key in ["inputBytes", "stepBytes", "totalStepBytes"] { assert_eq!(raised[key], defaults[key]); }
    for bytes in [0, 512 * MIB + 1] {
        assert_eq!(policy(bytes).validate().unwrap_err().code, "INVALID_RESOURCE_LIMITS");
    }
    let default = defaults["outputStepBytes"].as_u64().unwrap() as usize;
    agent_cad::resources::check_bytes("outputStepBytes", default, default, false).unwrap();
    reject(agent_cad::resources::check_bytes("outputStepBytes", default + 1, default, false).unwrap_err(), default);
}

fn cli(args: &[&str], source: &str) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agent-cad"))
        .args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let result = child.wait_with_output().unwrap();
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result.status.success(), receipt["ok"] == true);
    receipt
}

#[test]
fn library_cli_capabilities_and_active_host_limits_agree() {
    let library = agent_cad::capabilities();
    let default = cli(&["capabilities"], "");
    assert_eq!(library["resource_limits"]["outputStepBytes"], 32 * MIB);
    assert_eq!(library["limits"]["output_step_bytes"], 32 * MIB);
    assert_eq!(library["resource_limit_ceilings"]["outputStepBytes"], 512 * MIB);
    assert_eq!(default["data"]["resource_limits"], library["resource_limits"]);
    assert_eq!(default["data"]["resource_limit_ceilings"], library["resource_limit_ceilings"]);
    let raised = cli(&["capabilities", "--resource-limits", "{\"outputStepBytes\":67108864}"], "");
    assert_eq!(raised["ok"], true, "{raised}");
    assert_eq!(raised["data"]["limits"]["output_step_bytes"], 64 * MIB);
    assert_eq!(raised["data"]["resource_limits"]["inputBytes"], 32 * MIB);
}

#[test]
fn cli_build_cannot_publish_an_over_budget_artifact_on_either_path() {
    for assembly in [false, true] {
        let root = std::env::temp_dir().join(format!("colocad-output-red-{}-{assembly}", std::process::id()));
        assert!(!root.exists(), "fresh test output required");
        let out = root.to_str().unwrap();
        let result = cli(&["build", "--out", out, "--resource-limits", "{\"outputStepBytes\":1}"],
            &serde_json::to_string(&model(assembly)).unwrap());
        // Assert filesystem state first, including failed baseline runs.
        assert!(!root.exists(), "denied output must not publish a revision");
        assert_eq!(result["ok"], false);
        reject_json(&result["error"], 1);
    }
}

#[test]
fn model_data_cannot_authorize_output_and_output_permission_keeps_input_guards() {
    let mut raw = serde_json::to_value(model(false)).unwrap();
    raw["resourceLimits"] = json!({"outputStepBytes":64 * MIB});
    assert!(serde_json::from_value::<Model>(raw).is_err());
    let mut raw = serde_json::to_value(model(false)).unwrap();
    raw["outputStepBytes"] = json!(64 * MIB);
    assert!(serde_json::from_value::<Model>(raw).is_err());
    let limits: ResourceLimits = serde_json::from_value(json!({
        "stepBytes":1,"totalStepBytes":1,"outputStepBytes":64 * MIB
    })).unwrap();
    let mut raw = serde_json::to_value(model(false)).unwrap();
    raw["assets"] = json!({"source":{"step":"xx","sha256":"0".repeat(64)}});
    let source: Model = serde_json::from_value(raw).unwrap();
    let error = agent_cad::validate_with_resource_limits(&source, &limits).unwrap_err();
    assert_eq!(error.code, "RESOURCE_LIMIT");
    assert_eq!(error.details.unwrap()["resource"], "stepBytes");
}
