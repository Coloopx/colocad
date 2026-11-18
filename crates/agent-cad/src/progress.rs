//! [INPUT]: 宿主显式CLI开关与真实执行阶段。
//! [OUTPUT]: stderr上的单调有序JSON进展；不写stdout，不改变几何结果。
//! [POS]: 原生执行观测端口，不产生定时心跳。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use serde_json::json;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
static ENABLED: AtomicBool = AtomicBool::new(false);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}
pub fn emit_command(state: &str) {
    emit("command", state, None);
}
pub(crate) fn emit(phase: &str, state: &str, feature: Option<&str>) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let mut out = std::io::stderr().lock();
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut value = json!({"schema":"colocad.progress/1","seq":seq,"phase":phase,"state":state});
    if let Some(id) = feature {
        value["feature_id"] = json!(id);
    }
    let _ = writeln!(out, "COLOCAD_PROGRESS {}", value);
}
