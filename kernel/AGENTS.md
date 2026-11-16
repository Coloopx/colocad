# /kernel
> L2 | 父级: ../AGENTS.md

LICENSE: SDK分发许可证。
package.json: TypeScript编译工具及Node类型依赖。
tsconfig.json: 公开SDK源码到JavaScript与声明文件的编译合同。
catalog.json: 发布目录的能力索引。
release.json: 既有发行二进制摘要，本次源补丁不更新发行物。
src/: 宿主输入/STEP输出/报告策略及原生进程边界，见src/AGENTS.md。
dist/: 跟踪的JavaScript/声明文件，由同版 TypeScript 源码生成。
schemas/: 模型与扩展Schema；模型不能授权宿主额度。
licenses/: 分发依赖许可。
native/（生成目录）: 匹配源码的原生程序；独立构建验证，不随本次PR提交或发布。

SDK 接口变更后重新生成 dist，用对应源码编译的原生程序运行测试。输出与报告预算由宿主配置。

分平台的历史测试环境、二进制摘要和资源数据见 ../docs/resource-budgets.md。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
