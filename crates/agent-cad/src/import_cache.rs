//! [INPUT]: 已经验证的来源摘要、导入模式与宿主实体/定义/字节策略。
//! [OUTPUT]: 单次构建的模式化来源准入计数与最多一个中性来源模型槽。
//! [POS]: 单次构建最近一个已准入来源的复用；definition只读有界本地定义，part保留完整树拒绝；命中仅复用同SHA/模式的准入结果。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use crate::*;
use step_import::model::Model as SourceModel;

// -- 准入由缓存边界选择，definition不借用整树展开回调。 --
fn admit(asset: &StepAsset, definition: bool) -> Result<SourceModel> {
    if definition {
        return step_assembly::read_definitions(&asset.step);
    }
    let model = step_assembly::read(&asset.step)?.model;
    if model.assembly_instances > 0 {
        return Err(CadError::new("IMPORT_INCOMPLETE", "ordinary part import requires applied assembly transforms"));
    }
    Ok(model)
}

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
    ) -> Result<&SourceModel> {
        // 命中只复用已完成同模式准入的中性模型；BodyIdx/BRep始终由调用方独立处理。
        let same = self.recent.as_ref().is_some_and(|(sha, mode, _)| {
            sha == &asset.sha256 && *mode == definition
        });
        if same {
            self.hits += 1;
        } else {
            // 先丢弃旧模型再完整解析/准入；失败不能返回旧来源或同时保留两个来源。
            self.clear();
            self.parse_calls += 1;
            self.recent = Some((asset.sha256.clone(), definition, admit(asset, definition)?));
        }
        Ok(&self.recent.as_ref().expect("successful parse").2)
    }
    pub fn report(&self) -> Value {
        json!({"scope":"native_build_adjacent_part_definition_imports",
            "parse_calls":self.parse_calls,"hits":self.hits,"capacity":1,
            "retained_models":usize::from(self.recent.is_some())})
    }
}
