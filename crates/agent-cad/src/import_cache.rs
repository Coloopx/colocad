//! [INPUT]: 已经验证的来源摘要、导入模式和原有完整准入回调。
//! [OUTPUT]: 单次构建的解析计数与最多一个中性来源模型槽。
//! [POS]: 解析复用的诊断基线；本提交尚不复用，供Windows先记录真实红例。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::*;
use step_import::model::Model as SourceModel;

#[derive(Default)]
pub(crate) struct ImportCache {
    parse_calls: usize,
    hits: usize,
    recent: Option<(String, bool, SourceModel)>,
}
impl ImportCache {
    pub fn clear(&mut self) {
        self.recent = None;
    }
    pub fn parse(
        &mut self,
        asset: &StepAsset,
        definition: bool,
        read: impl FnOnce() -> Result<SourceModel>,
    ) -> Result<&SourceModel> {
        // 先丢弃旧模型再解析；失败不能返回旧来源，也不同时保留两个来源。
        self.clear();
        self.parse_calls += 1;
        self.recent = Some((asset.sha256.clone(), definition, read()?));
        Ok(&self.recent.as_ref().expect("successful parse").2)
    }
    pub fn report(&self) -> Value {
        json!({"scope":"native_build_adjacent_part_definition_imports",
            "parse_calls":self.parse_calls,"hits":self.hits,"capacity":1,
            "retained_models":usize::from(self.recent.is_some())})
    }
}
