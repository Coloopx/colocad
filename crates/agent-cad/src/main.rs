//! [INPUT]: 宿主字节/STEP计数参数与独立标准输入。[OUTPUT]: 有界CLI、STEP固定摘要和实际STEP/reportBytes回执及发布前双计量。
//! [POS]: 模型无权覆盖宿主预算。[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use agent_cad::{build_revision_with_resource_limits, capabilities, validate_with_resource_limits, CadError, Model};
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
    let count_limits = agent_cad::step_import_limits::host_args(&mut args)?;
    let limits = agent_cad::resources::take_host_limits(&mut args)?;
    agent_cad::step_import_limits::with_import_limits(count_limits, || {
    let result = run_args(args, limits).and_then(|data| {
        agent_cad::resources::check_success_report(&data, limits.report_bytes)?;
        Ok(data)
    });
    if let Err(error) = &result {
        agent_cad::resources::check_report(&json!({"ok":false,"error":error}), limits.report_bytes)?;
    }
    result
    })
}
fn run_args(mut args: Vec<String>, limits: agent_cad::ResourceLimits) -> Result<Value, CadError> {
    if args == ["capabilities"] {
        let mut report = capabilities();
        report["resource_limits"] = json!(limits);
        report["resource_limit_ceilings"] = json!(agent_cad::ResourceLimits::CEILINGS);
        report["limits"]["input_bytes"] = json!(limits.input_bytes);
        report["limits"]["step_asset_bytes"] = json!(limits.step_bytes);
        report["limits"]["step_total_bytes"] = json!(limits.total_step_bytes);
        report["limits"]["output_step_bytes"] = json!(limits.output_step_bytes);
        report["limits"]["report_bytes"] = json!(limits.report_bytes);
        report["query"]["limits"]["receipt_bytes"] = json!(limits.report_bytes);
        return Ok(report);
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
        limits.input_bytes.min(2 * 1024 * 1024)
    } else {
        limits.input_bytes
    };
    let mut bytes = vec![];
    io::stdin()
        .take((input_limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(fail)?;
    agent_cad::resources::check_bytes("inputBytes", bytes.len(), input_limit, true)?;
    if command == "query" {
        let request = serde_json::from_slice(&bytes)
            .map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
        return agent_cad::query::engine::execute_with_report_limit(Path::new(&args[2]), request, limits.report_bytes);
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
        let mut report = agent_cad::step_assembly::inspect_with_resource_limits(&request, &limits)?;
        report["resource_usage"]["input_bytes"] = json!(bytes.len());
        return Ok(report);
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
        report["artifact_directory"] = json!(args[2]);
        agent_cad::resources::check_success_report(&report, limits.report_bytes)?;
        agent_cad::resources::check_report_file(&report, limits.report_bytes)?;
        artifacts.insert(
            "report.json".into(),
            serde_json::to_vec_pretty(&report).map_err(fail)?,
        );
        publish(Path::new(&args[2]), artifacts)?;
        return Ok(report);
    }
    let model: Model =
        serde_json::from_slice(&bytes).map_err(|e| CadError::new("SCHEMA_ERROR", e.to_string()))?;
    validate_with_resource_limits(&model, &limits)?;
    if command == "validate" {
        return Ok(
            json!({"valid":true,"scope":"schema_and_dependencies_only","model_hash":agent_cad::model_hash(&model),
                "resource_limits":limits,"resource_usage":{"input_bytes":bytes.len(),"total_step_bytes":model.assets.values().map(|a|a.step.len()).sum::<usize>()}}),
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
    let mut result = build_revision_with_resource_limits(&model, &mut cache, queryable.then_some(true), &limits)?;
    result.report["resource_usage"]["input_bytes"] = json!(bytes.len());
    let step_sha256 = format!("{:x}", Sha256::digest(result.step.as_bytes()));
    // ---------- 固定摘要与STEP同一原子目录发布；reader无需加载report ----------
    result.artifacts.insert("model.step.sha256".into(), format!("{step_sha256}\n").into_bytes());
    result.report["artifact_sha256"] = json!({"model.step":step_sha256});
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
    result.report["artifact_directory"] = json!(args[2]);
    agent_cad::resources::check_success_report(&result.report, limits.report_bytes)?;
    agent_cad::resources::check_report_file(&result.report, limits.report_bytes)?;
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
        for (name, bytes) in files {
            let path = stage.join(&name);
            #[cfg(target_os = "linux")]
            if name == "model.step" && bytes.len() >= STEP_WRITE_WINDOW {
                write_large_step(&path, &bytes).map_err(fail)?;
                continue;
            }
            std::fs::write(path, bytes).map_err(fail)?;
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

#[cfg(target_os = "linux")]
const STEP_WRITE_WINDOW: usize = 4 * 1024 * 1024;

// The final exchange file can be larger than every other artifact combined.
// During private staging, release completed output pages as the file grows.
#[cfg(target_os = "linux")]
fn write_large_step(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::fd::AsRawFd;

    let mut output = std::fs::File::create_new(path)?;
    let mut offset = 0;
    for chunk in bytes.chunks(STEP_WRITE_WINDOW) {
        output.write_all(chunk)?;
        output.sync_data()?;
        // SAFETY: output owns this open descriptor; no pointers are passed.
        // Correctness depends on the write and sync, independently of advice support.
        unsafe {
            libc::posix_fadvise(
                output.as_raw_fd(),
                offset,
                chunk.len() as libc::off_t,
                libc::POSIX_FADV_DONTNEED,
            );
        }
        offset += chunk.len() as libc::off_t;
    }
    Ok(())
}
