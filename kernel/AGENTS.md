# /kernel
> L2 | 父级: ../AGENTS.md

LICENSE: SDK分发许可证。
package.json: TypeScript编译工具及Node类型依赖。
tsconfig.json: 公开SDK源码到JavaScript与声明文件的编译合同。
catalog.json: 发布目录的能力索引。
release.json: 既有发行二进制摘要，本次源补丁不更新发行物。
src/: 宿主输入/STEP输出/报告策略及原生进程边界，见src/AGENTS.md。
dist/: 跟踪的JavaScript/声明文件，由同版源码在Windows生成并逐字节同步。
schemas/: 模型与扩展Schema；模型不能授权宿主额度。
licenses/: 分发依赖许可。
native/（生成目录）: 匹配源码的原生程序；独立构建验证，不随本次PR提交或发布。

修改SDK接口时同步源码与由Windows生成的dist，保持原SDK默认输入/STEP权限；输出/报告授权来自宿主。已有发行二进制不能被当作新能力验收，测试须指定实际重建native。

独立验收：默认release opt3仅compile 2CPU/5GiB/no-swap成功（21m32s）；28个Rust test exe在2CPU/3GiB执行147/147，SDK full同quota为87/88，唯一SESSION_BUSY保持baseline；types/schema已通过。默认opt3和opt1/64两次cold 3GiB均OOM，不称3GiB冷编译通过。

实际重建native SHA：`b69fa92a93a65832cb0ff50634af11a75656c9d37ea6da15db54c94279cfdbc3`。验收必须显式选择该重建程序；跟踪dist按同版Windows生成字节同步。原platform binaries/release.json保持，不发布npm。compile/runtime分离recipe见../docs/resource-budgets.md；原helper内存gate不改。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
