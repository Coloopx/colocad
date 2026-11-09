# Specifications

Specifications distinguish proposed contracts from implemented behavior. A spec's acceptance matrix is not a test report; follow its status and linked evidence.

| Spec | Scope | Status |
| --- | --- | --- |
| [0001](0001-modeling-history.md) | Persistent sessions and CLI review | Implemented; synthetic turbofan scope superseded by 0002 |
| [0002](0002-industrial-actuator.md) | Catalog-based KK60 reconstruction and animation | Implemented with documented verification/limitations |
| [0003](0003-agent-guidance.md) | Modeling guidance, tool examples and release workflow | Delivered in alpha.5/alpha.6; no blind-agent success study |
| [0004](0004-engineering-query-and-assembly-interfaces.md) · [中文](0004-engineering-query-and-assembly-interfaces.zh-CN.md) | Exact geometry queries, mounting interfaces and assembly integration | **Proposed — implementation pending** |

The JSON documents under `examples/0004-*` illustrate proposed schemas. Their IDs/hashes/parameters are illustrative, and declaration/check files are model fragments, not executable alpha.6 inputs. They must be validated against generated runtime schemas when M0 is implemented.
