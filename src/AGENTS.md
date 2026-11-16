# /src
> L2 | 父级: ../AGENTS.md

build.mjs: 高层本地构建与产物管理。
hybrid-brep.mjs: 混合作者B-rep操作适配。
hybrid-mesh.mjs: 混合作者网格读写与布尔。
hybrid-step.mjs: 混合作者STEP输入读取。
hybrid.d.mts: 混合作者公开类型。
hybrid.mjs: 混合作者生命周期与交换。
index.d.mts: 产品入口和特征树类型。
index.mjs: 产品入口及公开SDK再导出。
render.mjs: 会话呈现与视频导出。
session-catalog.mjs: 会话SQLite目录及写锁。
session-payload.mjs: 不可变请求与压缩回执载荷。
session.d.mts: 会话类型及当前输出预算读取选项。
session.mjs: 会话生命周期和SHA/当前输出预算约束的bundle/HTML。
solidworks.mjs: 独立SolidWorks后端适配。
tree.mjs: 特征树编译。
view-server.mjs: 本地会话查看服务。

仅session输出读取消费kernel的ResourceLimits和readStepArtifact；不引入其它PR的库存/查询策略。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
