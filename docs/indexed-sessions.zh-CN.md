# alpha.9 开发版会话索引

新建项目时，`ModelingSession.create()` 和 `colocad record` 默认用本地 Rust 程序存取 SQLite。沙箱内的 agent 和 harness 照常调 SDK 或 CLI。源码检出后可用 `npm run build:kernel` 构建宿主平台的执行程序。[English](indexed-sessions.md)

```sh
node bin/colocad.mjs migrate ./old-project
node bin/colocad.mjs history ./old-project --limit 20
node bin/colocad.mjs history ./old-project --after 20 --limit 20
```

将上一页的 `nextCursor` 传给 `--after`，返回 null 时已到末页。SDK 对应 `session.history({after,limit})`。只需最近有效几何时，用 `session.latestGeometry()` 取记录，或用 `session.source()` 取经过哈希校验的源模型。编辑失败、查询和预览都保留在历史中。

旧项目在显式迁移前仍用 JSON。迁移逐条读取旧账本，保留原文件备份，最后原子切换 `session.json`。新格式中这个文件是目录指针。已有工具若依赖完整的 `steps` 数组，可调用 `session.read()` 或 `colocad history DIR --all`。这两个接口的内存需求随完整历史大小增长。需要直接读取旧 JSON 的应用仍可显式选择 `{storage:'json'}`。

在 Linux arm64、1 vCPU、512 MiB、无 swap 的实验中，项目目录使用工作区挂载。每个规模查询三次，表中查询值为 cgroup 总峰值中位数，迁移为单次。

| 合成历史条目 | alpha.8 查询历史基线 | 当前查询 | 当前迁移 |
| ---: | ---: | ---: | ---: |
| 1 万 | 57.12 MiB | 21.23 MiB | 41.54 MiB |
| 10 万 | 313.23 MiB | 21.40 MiB | 210.91 MiB |
| 50 万 | V8 堆耗尽 | 21.03 MiB | 512.00 MiB |

从 1 万到 50 万条，查询的采样匿名峰值增加 2.40 MiB，验收上限为 16 MiB。50 万条迁移耗时 23.0 秒，匿名峰值 18.45 MiB，文件页采样峰值 492.97 MiB，内核 OOM 次数为零。迁移的容器总占用已触及上限，仍需给同一沙箱中的 agent 和 harness 预留空间。

真实写入测试在初始建模后完成 100 次成功改参和 9,900 次成功几何查询。全程峰值 271.67 MiB，重新打开后按顺序检查全部 10,001 条记录与已接受几何。恢复测试覆盖迁移和建模途中 SIGKILL。实体几何落盘、流式装配 STEP 和查看器分页属于后续阶段。[原始数据与复现](../benchmarks/session-catalog/README.md)
