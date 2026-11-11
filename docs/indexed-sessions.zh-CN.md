# alpha.9 开发版会话索引

新建项目时，`ModelingSession.create()` 和 `colocad record` 默认用本地 Rust 程序存取 SQLite。沙箱内的 agent 和 harness 照常调 SDK 或 CLI。源码检出后可用 `npm run build:kernel` 构建宿主平台的执行程序。[English](indexed-sessions.md)

```sh
node bin/colocad.mjs migrate ./old-project
node bin/colocad.mjs history ./old-project --limit 20
node bin/colocad.mjs history ./old-project --after 20 --limit 20
```

将上一页的 `nextCursor` 传给 `--after`，返回 null 时已到末页。SDK 对应 `session.history({after,limit})`。只需最近有效几何时，用 `session.latestGeometry()` 取记录，或用 `session.source()` 取经过哈希校验的源模型。编辑失败、查询和预览都保留在历史中。

旧项目在显式迁移前仍用 JSON。迁移逐条读取旧账本，写入私有临时数据库；每 4,096 条或约 4 MiB 条目提交一次。Linux 上会提示内核回收已读输入和已落盘的新增数据库页。保留原文件备份，最后原子切换 `session.json`。新格式中这个文件是目录指针。已有工具若依赖完整的 `steps` 数组，可调用 `session.read()` 或 `colocad history DIR --all`。这两个接口的内存需求随完整历史大小增长。需要直接读取旧 JSON 的应用仍可显式选择 `{storage:'json'}`。

在 Linux arm64、1 vCPU、512 MiB、无 swap 的实验中，项目目录使用工作区挂载。表中是整个 cgroup 的总峰值；查询均为三次中位数。新版迁移的 1 万和 10 万条各测一次，50 万条为三次中位数。

| 合成历史条目 | alpha.8 查询历史基线 | 当前查询 | 当前迁移 |
| ---: | ---: | ---: | ---: |
| 1 万 | 57.12 MiB | 21.23 MiB | 30.63 MiB |
| 10 万 | 313.23 MiB | 21.40 MiB | 31.75 MiB |
| 50 万 | V8 堆耗尽 | 21.03 MiB | 33.06 MiB |

从 1 万到 50 万条，查询的采样匿名峰值增加 2.40 MiB，验收上限为 16 MiB。50 万条的同条件复测中，旧版和新版迁移耗时中位数分别为 25.63 秒和 25.44 秒，采样文件页峰值中位数由 492.97 降到 14.19 MiB；新版三轮都没有触发限额回收。另用 64 MiB 无 swap 容器完成同一迁移，总峰值 33.45 MiB。以上不含常驻 agent 和 harness；文件系统可忽略回收提示，超大单条记录和元数据头也会增加内存，磁盘仍须容纳源文件和数据库。[对照方法和逐次记录](../benchmarks/session-catalog/migration-memory.md)。

真实写入测试在初始建模后完成 100 次成功改参和 9,900 次成功几何查询。全程峰值 271.67 MiB，重新打开后按顺序检查全部 10,001 条记录与已接受几何。恢复测试覆盖迁移和建模途中 SIGKILL。实体几何落盘、流式装配 STEP 和查看器分页属于后续阶段。[原始数据与复现](../benchmarks/session-catalog/README.md)

使用 alpha.9 候选包时，先用 `colocad head SESSION` 读取已接受步骤及查询版本，再用 `colocad inspect SESSION REQUEST.json` 保存精确查询的请求和回执。改参使用 `colocad edit SESSION PARAMETERS.json`。一次查询可以在同一零件、坐标系与公差下测量多个特征。查询步骤通过 `inputFile` 引用已接受的源文件，读取时校验 SHA-256。[四孔改六孔的实例](../skills/colocad/references/modeling-loop.md#inspect-and-edit-from-a-shell)
