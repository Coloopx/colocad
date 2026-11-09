# 精确查询与几何基准装配

**当前为开发源码能力，已发布的 alpha.6 原生包尚不包含本次变更。** 请使用同一份 checkout 编译的内核和 SDK。[English](engineering-queries.md) · [可运行例子](../examples/engineering-queries/README.md) · [详细规范](../specs/0004-engineering-query-and-assembly-interfaces.zh-CN.md)。

这层能力用于在装配前查询零件的安装几何。几何解释在本地 Rust 内核中完成，SDK、CLI 与 agent 工具共用实现；agent、harness 和可执行文件仍位于同一沙箱中。

## 构建一次，查询已接受版本

`cad.build(model, {queryable:true})` 返回 `artifact_directory` 和 `query_revision_id`，并保存 `query-manifest.json` 与精确 `query-definition-*.smb`。将带预期版本号、目标、坐标系、容差和查询列表的请求交给 `cad.query(directory, request)`。完整 SDK 代码见英文页。

查询不重放建模特征、不读 STEP、不导出 STEP、不修补几何、不生成网格。整个工件目录迁移或换进程重开后，快照引用保持一致。普通模型按需启用；声明 `Model.datums` 的模型自动启用，显式关闭会报错。SDK 的 `queryRoots` 由宿主配置，默认只允许 `outputRoot` 下的工件。

```sh
colocad record model.json session --queryable --label '可查询零件'
# 采用构建回执里的 artifact_directory 与 query_revision_id。
colocad query ARTIFACT_DIRECTORY request.json --out receipt.json
colocad view session
```

CLI 的 `--out` 拒绝覆盖已有文件。`colocad/1` 特征树也可使用 `colocad build tree.json NEW_DIR --queryable`。`ModelingSession.query(request)` 保存查询输入、回执和源版本，不替换已接受几何，失败查询同样留档。session 工具自动绑定最后接受的几何工件；原始 SDK 工具显式接收受宿主限制的工件路径。目前可检查历史记录，尚无查询拓扑高亮叠层。

定义坐标使用 `target:{definition_id:'part'}`；装配坐标使用 `target:{instance_path:['group','part_occurrence']}` 与 `space:'assembly'`。位置包含平移，方向不包含平移。查询采用求解后的正常装配位置，不随爆炸预览姿态变化。

## 选择表达意图，歧义必须保留

支持实体枚举、过滤、子拓扑、祖先、相邻拓扑、集合运算、排序和数量断言。筛选项包括名称、解析几何类型、面积/长度/半径范围、平面法向、内外环，以及有界的 `all/any/not`。

安装面上的孔中心应按“指定平面 → 内环 → 圆边 → 圆心”查询，避免把背面的开口也选进来。`siblings` 显式指定共享子拓扑，例如通过边相邻的面：`{kind:'siblings',input:selectedFace,entity:'face',via:'edge'}`。

排序并列时返回整个并列集合，随后 `expect:1` 必须失败；分页不能把歧义隐藏成唯一结果。集合按精确拓扑身份去重，环内重复使用的接缝边保留。游标绑定版本、目标、坐标系、容差和操作。快照引用用于观察；改参后旧引用会失效，持久化意图应使用语义选择配方。

限制：每批 64 项查询、每页 256 项、表达式深度 8 / 128 节点、20 万次邻接访问、2 MiB 输入、8 MiB 输出、64 MiB 精确快照。它们是输入边界，不能据此宣称 512 MiB 沙箱已验收。源数据未声明的工程精度仍是未知。

## 把基准直接用于配合

`Model.datums` 的声明包含 `id`、最终零件 `definition`、`tolerance` 和 `derive`。名称在同一定义中唯一；输出类型由推导规则确定。`selected` 配方使用通用选择，必须得到唯一实体，然后推导顶点/圆心、解析平面、圆柱/圆轴或直边方向。

其他配方包括显式设计点 `design_point`、有序点差 `point_difference` 和完整 `frame`。frame 引用一个原点基准及 x/z 方向基准，每个方向显式选择正负；检查非退化和正交，构成右手系。依赖必须是同一定义中较早的声明；不能使用旧快照引用。回执区分几何推导与显式设计意图。

附件可以写成 `{instance:'part_occurrence',datum:'seat'}`。平面配合需要 plane，同心需要 axis，点重合/距离需要 point；固定、滑动和转动关节需要完整 frame。完整 frame 也可提供低维附件。`frame` 与 `datum` 互斥；旧式附件全部省略时仍采用原有默认 frame。

内部补出的数值基底不会增加工程约束。轴线加轴向平面保留绕轴的一个自由度；单独的圆柱轴线没有转动关节所需的轴向原点，需补足点/frame 或组合轴和平面约束。

通过 `{id:'mount',op:'named_datums',ids:['seat']}` 回读命名基准。回执中的源拓扑引用绑定最终版本，并按目标实例变换。改参重建会重算配方；缺失、歧义、退化或类型不兼容均返回结构化错误。

## 验收范围

例子将 OCCT 独立生成的法兰 STEP 导入，选择孔轴和底面，用几何基准装配轴，导出 STEP；过粗的轴被精确碰撞检查拒绝。独立 OCCT 回读检查两个实体、解析体积及 0.2 mm 最小间隙。这是通用解析验证件，不是商业产品复原，也不是工业模型成功率语料。

尚未完成：引用命名基准的相对筛选、全部周期/奇异点专用描述、孔和沉孔识别、孔组与安装接口、通用零件替换验收、困难模型可靠性语料和 Linux 资源验收。装配包围盒由定义 AABB 刚性变换得到，旋转后可能偏宽。查询结果不证明加工精度、螺纹、承载能力或局部求解的全局唯一性。[四阶段目标与后续自优化循环](../specs/0005-capability-reliability-and-optimization.zh-CN.md)。
