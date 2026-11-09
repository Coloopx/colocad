use agent_cad::{
    build_revision_with_options, capabilities, validate, CadError, Model, MAX_INPUT_BYTES,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;
fn fail(e: impl std::fmt::Display) -> CadError {
    CadError::new("IO_ERROR", e.to_string())
}
fn main() {
    let result = std::panic::catch_unwind(run).unwrap_or_else(|_| {
        Err(CadError::new(
            "KERNEL_PANIC",
            "local CAD process aborted; previous artifacts remain unchanged",
        ))
    });
    match result {
        Ok(data) => println!("{}", json!({"ok":true,"data":data})),
        Err(error) => {
            println!("{}", json!({"ok":false,"error":error}));
            std::process::exit(1);
        }
    }
}
fn run() -> Result<Value, CadError> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["capabilities"] {
        return Ok(capabilities());
    }
    if args == ["schema"] {
        return serde_json::to_value(schemars::schema_for!(Model)).map_err(fail);
    }
    if args == ["query-schema"] {
        return serde_json::to_value(schemars::schema_for!(agent_cad::query::contract::Request))
            .map_err(fail);
    }
    let queryable =
        args.first().is_some_and(|a| a == "build") && args.iter().any(|a| a == "--queryable");
    if queryable {
        let at = args.iter().position(|a| a == "--queryable").unwrap();
        args.remove(at);
    }
    let command = args.first().map(String::as_str).unwrap_or("");
    let incremental = command == "build"
        && args.get(1).map(String::as_str) == Some("--out")
        && (args.len() == 4 && args[3] == "--checkpoints"
            || args.len() == 5 && args[3] == "--reuse");
    if !incremental
        && !(args == ["solve-sketch"]
            || args == ["inspect-step"]
            || args == ["validate"]
            || (command == "query" && args.len() == 3 && args[1] == "--snapshot")
            || (matches!(command, "build" | "observe") && args.len() == 3 && args[1] == "--out"))
        && !(command == "observe"
            && args.len() == 5
            && args[1] == "--out"
            && args[3] == "--options")
    {
        return Err(CadError::new(
            "USAGE",
            "agent-cad capabilities | schema | validate < model.json | solve-sketch < request.json | inspect-step < asset.json | build --out NEW_DIR < model.json | observe --out NEW_DIR < mesh-request.json",
        ));
    }
    let input_limit = if command == "query" {
        2 * 1024 * 1024
    } else {
        MAX_INPUT_BYTES
    };
    let mut bytes = vec![];
    io::stdin()
        .take((input_limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(fail)?;
    if bytes.len() > input_limit {
        return Err(CadError::new(
            "RESOURCE_LIMIT",
            "input exceeds command byte budget",
        ));
    }
    if command == "query" {
        let request = serde_json::from_slice(&bytes)
            .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
        return agent_cad::query::engine::execute(Path::new(&args[2]), request);
    }
    if command == "solve-sketch" {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            sketch: agent_cad::sketch::Sketch,
            #[serde(default)]
            parameters: std::collections::BTreeMap<String, f64>,
        }
        let request: Request = serde_json::from_slice(&bytes)
            .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
        return serde_json::to_value(agent_cad::sketch::solve(
            &request.sketch,
            &request.parameters,
        )?)
        .map_err(fail);
    }
    if command == "inspect-step" {
        let request: agent_cad::StepAsset = serde_json::from_slice(&bytes)
            .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
        return agent_cad::step_assembly::inspect(&request);
    }
    if command == "observe" {
        let started = std::time::Instant::now();
        let request: agent_cad::preview::ObserveRequest = if args.len() == 5 {
            agent_cad::preview::ObserveRequest {
                scene: serde_json::from_slice(&bytes)
                    .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?,
                options: serde_json::from_str(&args[4])
                    .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?,
            }
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?
        };
        let decode_ms = started.elapsed().as_secs_f64() * 1000.;
        let (mut artifacts, mut report) = agent_cad::preview::observe(&request)?;
        report["timing_ms"]["cache_decode"] = json!(decode_ms);
        report["artifact_sha256"] = json!(artifacts
            .iter()
            .map(|(name, data)| (name.clone(), format!("{:x}", Sha256::digest(data))))
            .collect::<std::collections::BTreeMap<_, _>>());
        artifacts.insert(
            "report.json".into(),
            serde_json::to_vec_pretty(&report).map_err(fail)?,
        );
        publish(Path::new(&args[2]), artifacts)?;
        return Ok(report);
    }
    let model: Model =
        serde_json::from_slice(&bytes).map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
    validate(&model)?;
    if command == "validate" {
        return Ok(
            json!({"valid":true,"scope":"schema_and_dependencies_only","model_hash":agent_cad::model_hash(&model)}),
        );
    }
    let out = Path::new(&args[2]);
    if out.exists() {
        return Err(CadError::new(
            "OUTPUT_EXISTS",
            "use a new revision directory",
        ));
    }
    let mut cache = if incremental {
        agent_cad::incremental::Checkpoints::revision(args.get(4).map(Path::new))?
    } else {
        Default::default()
    };
    let mut result = build_revision_with_options(&model, &mut cache, queryable.then_some(true))?;
    result.report["artifact_sha256"] =
        json!({"model.step":format!("{:x}",Sha256::digest(result.step.as_bytes()))});
    for (name, bytes) in &result.artifacts {
        result.report["artifact_sha256"][name] = json!(format!("{:x}", Sha256::digest(bytes)));
    }
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        if let Some(rss) = status.lines().find(|l| l.starts_with("VmHWM:")) {
            result.report["process_peak_rss_kib"] = json!(rss
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<u64>().ok()));
        }
    }
    let mut files = result.artifacts;
    files.insert("model.step".into(), result.step.into_bytes());
    files.insert(
        "model.json".into(),
        serde_json::to_vec_pretty(&model).map_err(fail)?,
    );
    files.insert(
        "report.json".into(),
        serde_json::to_vec_pretty(&result.report).map_err(fail)?,
    );
    publish(out, files)?;
    Ok(result.report)
}

fn publish(out: &Path, files: std::collections::BTreeMap<String, Vec<u8>>) -> Result<(), CadError> {
    if out.exists() {
        return Err(CadError::new(
            "OUTPUT_EXISTS",
            "use a new revision directory",
        ));
    }
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(fail)?;
    let name = out
        .file_name()
        .ok_or_else(|| CadError::new("INVALID_OUTPUT", "output must name a new directory"))?
        .to_string_lossy();
    let stage = parent.join(format!(".{name}.{}.staging", std::process::id()));
    std::fs::create_dir(&stage).map_err(fail)?;
    let publish = (|| -> Result<(), CadError> {
        for (name, bytes) in &files {
            std::fs::write(stage.join(name), bytes).map_err(fail)?;
        }
        if out.exists() {
            return Err(CadError::new(
                "OUTPUT_EXISTS",
                "output appeared during build",
            ));
        }
        std::fs::rename(&stage, out).map_err(fail)?;
        Ok(())
    })();
    if publish.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    publish?;
    Ok(())
}
