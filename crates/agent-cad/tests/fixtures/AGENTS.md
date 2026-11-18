# /crates/agent-cad/tests/fixtures
> L2 | 父级: ../AGENTS.md

nested-assembly.step: 原独立OCCT嵌套装配输入。
occt-drilled-block.step: 原独立OCCT带孔块输入。
output-budget-box.json: 2×3×4mm公开小盒体，单体24mm³/两实例48mm³，供输出预算边界及缓存回放。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md

planar_conic_seam.rs: 自有17度seam同步换基及单顶点不一致生成器，测试运行时产生STEP。

washer129-colocad.step: 公开原生boss等价washer129，半径4.5/2.2×厚1.5mm、12mm间距16列；非维护者原字节，SHA与recipe见washer129-colocad.json。
washer129-colocad.json: 固定公开recipe、来源SHA及明确的等价/非exact provenance。
