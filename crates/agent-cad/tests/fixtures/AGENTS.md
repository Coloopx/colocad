# /crates/agent-cad/tests/fixtures
> L2 | 父级: ../AGENTS.md

nested-assembly.step: 原独立OCCT嵌套装配输入。
occt-drilled-block.step: 原独立OCCT带孔块输入。
output-budget-box.json: 2×3×4mm公开小盒体，单体24mm³/两实例48mm³，供输出预算边界及缓存回放。

uv-rescaled-plate.step: uv_trim_metric.rs::plate生成的20x20x6mm、半径1mm通孔块；两端面的UV范围为[0,1e-5]²，三维边/顶点/容差保留，用于公开SDK复现。

planar_conic_seam.rs: 自有17度seam同步换基及单顶点不一致生成器，测试运行时产生STEP。

washer129-colocad.step: 公开原生boss等价washer129，半径4.5/2.2×厚1.5mm、12mm间距16列；非维护者原字节，SHA与recipe见washer129-colocad.json。
washer129-colocad.json: 固定公开recipe、来源SHA及明确的等价/非exact provenance。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
