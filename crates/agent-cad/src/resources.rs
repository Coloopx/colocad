//! [INPUT]: 宿主CLI或Rust调用方的显式字节预算，不读取模型字段。
//! [OUTPUT]: 输入/库存与独立outputStepBytes策略、固定硬顶及准确或下界超限回执。
//! [POS]: SDK/native同一宿主资源合同；模型无权授权，几何和实体数独立。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::*;

const MIB: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ResourceLimits {
    pub input_bytes: usize,
    pub step_bytes: usize,
    pub total_step_bytes: usize,
    pub output_step_bytes: usize,
}
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            input_bytes: MAX_INPUT_BYTES,
            step_bytes: 24 * MIB,
            total_step_bytes: 24 * MIB,
            output_step_bytes: 32 * MIB,
        }
    }
}
impl ResourceLimits {
    pub const CEILINGS: Self = Self {
        input_bytes: 512 * MIB,
        step_bytes: 256 * MIB,
        total_step_bytes: 512 * MIB,
        output_step_bytes: 512 * MIB,
    };
    pub fn validate(&self) -> Result<()> {
        for (name, value, ceiling) in [
            ("inputBytes", self.input_bytes, Self::CEILINGS.input_bytes),
            ("stepBytes", self.step_bytes, Self::CEILINGS.step_bytes),
            ("outputStepBytes", self.output_step_bytes, Self::CEILINGS.output_step_bytes),
            (
                "totalStepBytes",
                self.total_step_bytes,
                Self::CEILINGS.total_step_bytes,
            ),
        ] {
            if value == 0 || value > ceiling {
                let mut e = CadError::new(
                    "INVALID_RESOURCE_LIMITS",
                    "byte budget outside host ceiling",
                );
                e.details =
                    Some(json!({"resource":name,"requested_bytes":value,"ceiling_bytes":ceiling}));
                return Err(e);
            }
        }
        if self.step_bytes > self.total_step_bytes || self.total_step_bytes > self.input_bytes {
            return Err(CadError::new(
                "INVALID_RESOURCE_LIMITS",
                "stepBytes <= totalStepBytes <= inputBytes required",
            ));
        }
        Ok(())
    }
}

pub fn check_bytes(
    resource: &'static str,
    observed: usize,
    limit: usize,
    lower_bound: bool,
) -> Result<()> {
    if observed <= limit {
        return Ok(());
    }
    let mut e = CadError::new(
        "RESOURCE_LIMIT",
        format!("{resource} exceeds {limit} bytes"),
    );
    e.details = Some(
        json!({"resource":resource,"observed_bytes":observed,"limit_bytes":limit,
        "observed_is_lower_bound":lower_bound}),
    );
    Err(e)
}

pub fn take_host_limits(args: &mut Vec<String>) -> Result<ResourceLimits> {
    let indices = args
        .iter()
        .enumerate()
        .filter_map(|(i, a)| (a == "--resource-limits").then_some(i))
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Ok(ResourceLimits::default());
    }
    if indices.len() != 1 || indices[0] + 1 >= args.len() {
        return Err(CadError::new(
            "INVALID_RESOURCE_LIMITS",
            "one --resource-limits JSON option required",
        ));
    }
    let at = indices[0];
    let raw = args.remove(at + 1);
    args.remove(at);
    if raw.len() > 4096 {
        return Err(CadError::new(
            "INVALID_RESOURCE_LIMITS",
            "host option exceeds 4096 bytes",
        ));
    }
    let limits: ResourceLimits = serde_json::from_str(&raw)
        .map_err(|e| CadError::new("INVALID_RESOURCE_LIMITS", e.to_string()))?;
    limits.validate()?;
    Ok(limits)
}
