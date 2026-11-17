# /kernel/src
> L2 | 父级: ../../AGENTS.md

index.ts: package公开入口与宿主资源合同导出。
types.ts: 配置/回执类型，模型类型来自generated。
resource-limits.ts: 默认/有限硬上限、不可变宿主字节策略和结构化超限错误。
assets.ts: 无路径的原文SHA注册与单源字节预算。
client.ts: 公开SDK与原生host选项传递、准确JSON字节预算。
process.ts: 无shell子进程、超时/输出限制和回执解码。
artifacts.ts: 独占且原子的本地产物目录。
binary.ts: 本平台原生内核定位。
files.ts: 宿主模型文件读写与STEP注册。
generated.ts: Model/Feature的原生Schema派生类型。
query-generated.ts: 精确查询原生合同派生类型。
helpers.ts: 有限声明式建模辅助器。
preview-cache.ts: 固定预览缓存及显示预算。
tools.ts: Agent工具公开合同与SDK调用适配。

[PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
outputStepBytes在resource-limits.ts提供独立默认/硬顶；files.ts增加readStepArtifact的当前宿主输出预算与固定摘要核验，index.ts导出。更新跟踪的 dist 后使用当前工作树编译的原生程序验收。

reportBytes：全命令invoke/inspect/preview report按当前host报告额度；process保留UTF8字节有界接收，files提供校验reader。默认8MiB/顶128MiB/最小1024B，STEP固定SHA reader不读report。

独立验收的编译/运行分离：默认release opt3在2CPU/5GiB/no-swap仅编译成功，随后Rust147/147在2CPU/3GiB运行；两次cold 3GiB OOM记录保留。详见../../docs/resource-budgets.md，不更新platform/release.json或发布npm。

progress.ts: 有界有序原生事件解析，重放和普通stderr日志不续期。
