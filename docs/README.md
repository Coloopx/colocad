# Developer documentation

[Actual modeling workflow](modeling-workflow.zh-CN.md) · [CLI modeling history and viewer (development)](modeling-history.md)

Build locally with **SDK → CLI/tools → skill**. The skill guides modeling decisions; the SDK and Rust kernel execute them. Agent, harness and CAD process share the same sandbox and filesystem.

These guides target **@coloopx/colocad 0.1.0-alpha.3**, Node.js 22+, Linux x64/arm64 or macOS arm64. The project is still alpha; documentation does not imply complete CAD coverage.

[中文开发指南](README.zh-CN.md)

| Your task | Start here |
| --- | --- |
| Create a part and edit a dimension | [Getting started](getting-started.md) |
| Choose the model format and understand editability | [Models and artifacts](concepts.md) |
| Call the SDK, CLI or tools | [API reference](api.md) |
| Integrate with an existing sandbox/harness | [Integration architecture](integrations/overview.md) |
| Register tools in Pi agent core or Pi coding agent | [Pi integration](integrations/pi.md) |
| Use Claude Code | [Claude Code integration](integrations/claude-code.md) |
| Use Codex | [Codex integration](integrations/codex.md) |
| Model efficiently from dimensions or views | [Best practices](best-practices.md) |
| Diagnose a failed call or installation | [Troubleshooting](troubleshooting.md) |
| Run examples and contribute a change | [Contributing](../CONTRIBUTING.md) |
| Understand what has actually been tested | [Integration verification](integrations/verification.md), [geometry validation](validation.md) |

Runnable adapters and their regression tests live in [examples/integrations](../examples/integrations/README.md). They install the published npm release. This example project is in GitHub; it is not included in the alpha.3 npm tarball. The skill, assets and its helper scripts **are** included.

Performance depends on workload and execution mode. See the [README measurements](../README.md#how-many-tendon-hands-can-an-agent-deliver-in-100-seconds), with their startup, geometry and memory accounting, before choosing a budget. A 512 MiB CAD benchmark is not a 512 MiB guarantee for a resident LLM harness plus CAD.
