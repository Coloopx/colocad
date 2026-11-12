# 自由曲面与网格混合装配

有机外形可以直接保留为扫描或生成的闭合网格，支架、转轴和光学安装面可以继续使用精确 CAD。需要可改截面的外壳时，也可以从对应的三维轮廓点建立光顺放样。用 `colocad/hybrid/1` 选择本地混合执行路径，精确曲面计算使用 OCCT WASM，三角实体布尔使用 Manifold WASM。常规机械模型继续走 Rust/SolidCore。

```sh
npm install @coloopx/colocad@0.1.0-alpha.10
npx colocad build node_modules/@coloopx/colocad/examples/hybrid-enclosure/model.json ./housing-build
```

打开 `housing-build/review.html` 可以离线旋转、切开模型、查看材质和检查报告。修改原始 JSON 里的转轴角度，指定新目录再执行，即可验证另一姿态。SDK 可调用 `buildHybrid(spec, directory, {assetRoot})`，JSON Schema 位于 `@coloopx/colocad/hybrid/schema`。

1. 先在多视图上对应吻端、颊部、轮廓转折和安装基准，单独确定高度与纵深。每圈截面按相同起点和方向填写同数目的三维控制标志点，避免沿用与参考图冲突的包络。先看正面、侧面和俯视轮廓，再加安装细节。
2. `profile_loft` 通过各站点的光顺闭合轮廓。可分别构建上下颚。用 `shell` 指定开口平面和向内偏置厚度，用 `hollow` 做闭合空腔，或用 `cut` 对两个光顺放样作差。偏置受局部曲率与自交限制，失败会返回特征 ID。
3. `import_mesh` 接收 STL、OBJ 和静态三角 GLB。STL/OBJ 显式给单位，GLB 按米与 Y 向上坐标系导入。与 `import_step` 的机械实体共用实例位姿。实例显式 `material.color` 和 `opacity` 会进入 HTML 与 GLB。纹理、骨骼动画和 OBJ 材质库另行烘焙或指定静态材质。
4. 为干涉检查指定两个真实实例，收据给出算法和交叠体积。精确件之间用 B-rep 布尔交集，涉及网格时用实际三角实体。在空腔里放一件，再故意移穿壳壁，分别检查零值和正值。多个离散姿态的验收只覆盖这些角度。
5. 用沿壁面法向放置的 `wall_probe` 测局部材料厚度。`slice` 返回截面面积和轮廓数，`overhang` 给出当前打印朝向的下悬表面积。实际支撑、桥接和打印参数仍需切片软件验证。

默认导出完整 `assembly.glb`、各实例 STL、精确件 `exact.step` 和单文件 HTML。启用 `facetedStep:true` 会另写网格的平面分面 B-rep，以及包含全部实例的 `assembly.step`。实体在 STEP 中处于装配位置，编辑参数、姿态和约束保存在源文档。需要对网格本身做精确切割时，可以显式 `faceted_brep` 转换，三角面数会直接影响开销。分面写入当前支持独立外壳，遇到封闭内负壳会给出错误。

[全部操作、限制与复现命令](hybrid.md) · [发布验收数据](evidence/release-alpha.10.json) · [本次全量复测](benchmark-alpha10.zh-CN.md)
