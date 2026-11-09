# Specifications

Specifications distinguish proposed contracts from implemented behavior. A spec's acceptance matrix is not a test report; follow its status and linked evidence.

| Spec | Scope | Status |
| --- | --- | --- |
| [0001](0001-modeling-history.md) | Persistent sessions and CLI review | Implemented; synthetic turbofan scope superseded by 0002 |
| [0002](0002-industrial-actuator.md) | Catalog-based KK60 reconstruction and animation | Implemented with documented verification/limitations |
| [0003](0003-agent-guidance.md) | Modeling guidance, tool examples and release workflow | Delivered in alpha.5/alpha.6; no blind-agent success study |
| [0004](0004-engineering-query-and-assembly-interfaces.md) · [中文](0004-engineering-query-and-assembly-interfaces.zh-CN.md) | Exact geometry queries, datums, mounting interfaces and assembly integration | **Revision 2 — six review findings incorporated; staged implementation and [experiments](experiments/0004/README.md) in progress** |
| [0005](0005-capability-reliability-and-optimization.md) · [中文](0005-capability-reliability-and-optimization.zh-CN.md) | Ordered functional coverage → reliability → usability → benchmark/retrospective optimization | **G1 active; G2–G4 gated** |

The JSON documents under `examples/0004-*` illustrate proposed schemas. Their IDs/hashes/parameters are illustrative, and declaration/check files are model fragments, not executable alpha.6 inputs. Validate each layer against generated runtime schemas when that layer is implemented. Runnable experimental inputs live under `experiments/0004`; the ledger distinguishes implemented source capabilities from proposals and published npm capabilities.
