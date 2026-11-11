# Developer documentation

[Current three-backend measurements and assembly phase breakdown](benchmark-bounded-candidate.md)

Release measurements: [alpha.9 benchmark](benchmark-alpha9.md) · [中文](benchmark-alpha9.zh-CN.md) · [single-file HTML](benchmarks/alpha9/scorecard.html).

[Actual modeling workflow](modeling-workflow.zh-CN.md) · [CLI modeling history and viewer](modeling-history.md)

Build locally with **SDK → CLI/tools → skill**. The skill guides modeling decisions; the SDK and Rust kernel execute them. Agent, harness and CAD process share the same sandbox and filesystem.

These guides target **@coloopx/colocad 0.1.0-alpha.9**, Node.js 22+, Linux x64/arm64 or macOS arm64. The project is still alpha; documentation does not imply complete CAD coverage.

For alpha.9, see [indexed sessions, history pagination and migration](indexed-sessions.md).

[中文开发指南](README.zh-CN.md)

| Your task | Start here |
| --- | --- |
| Prepare materials, design tools/skills/prompts and tune an agent workflow | [End-to-end handbook](agent-modeling-handbook.md), [research basis](agent-modeling-research.md) |
| Create a part and edit a dimension | [Getting started](getting-started.md) |
| Choose the model format and understand editability | [Models and artifacts](concepts.md) |
| Call the SDK, CLI or tools | [API reference](api.md) |
| Integrate with an existing sandbox/harness | [Integration architecture](integrations/overview.md) |
| Register tools in Pi agent core or Pi coding agent | [Pi integration](integrations/pi.md) |
| Use Claude Code | [Claude Code integration](integrations/claude-code.md) |
| Use Codex | [Codex integration](integrations/codex.md) |
| Model efficiently from dimensions or views | [Best practices](best-practices.md) |
| Query, declare installation intent, replace a part and verify edits | [Practical workflow and runnable example](best-practices.md#query-declare-intent-assemble-then-challenge-the-edit), [query API](engineering-queries.md) |
| Diagnose a failed call or installation | [Troubleshooting](troubleshooting.md) |
| Run examples and contribute a change | [Contributing](../CONTRIBUTING.md) |
| Compare execution performance and limits | [Performance evidence](performance.md) |
| Control memory as parts/history grow | [Three-backend memory study](memory-persistence-report.md), [alpha.9 persistence project](../specs/0006-bounded-memory-persistence.md) |
| Check native SOLIDWORKS export status | [Native author](native-solidworks.md) |
| Understand what has actually been tested | [Integration verification](integrations/verification.md), [geometry validation](validation.md) |
| Check G1–G3 acceptance, Linux limits and live-agent traces | [Frozen corpus, lifecycle and sandbox evidence](../specs/experiments/0005/README.md) |
| Contribute geometry queries and assembly interfaces | [SDD 0004 revision 2 — contract and 28 acceptance cases](../specs/0004-engineering-query-and-assembly-interfaces.md), [implementation evidence](../specs/experiments/0004/README.md) |

Runnable adapters and their regression tests live in [examples/integrations](../examples/integrations/README.md). The standalone SDK example pins the published alpha.3 compatibility baseline; current Pi/session adapters use this checkout and the matching native. The complete Pi example project is in GitHub. Alpha.5 also packages the compact adapter and guided workflow example. The skill, assets and its helper scripts **are** included.

Performance depends on workload and execution mode. See the [README measurements](../README.md#performance), with their startup, geometry and memory accounting, before choosing a budget. A 512 MiB CAD benchmark is not a 512 MiB guarantee for a resident LLM harness plus CAD.

- [Exact geometry queries and datum-driven assembly (alpha.7)](engineering-queries.md)
- [Four ordered goals and benchmark/retrospective optimization](../specs/0005-capability-reliability-and-optimization.md)

## Coverage and roadmap

- [CadQuery coverage and design comparison](cadquery-comparison.md), including the 150-entry mapping.
- [ROADMAP](../ROADMAP.md): architecture, accepted scope, planned benchmarks and RSI promotion rules.
