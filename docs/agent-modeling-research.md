# Research basis and evidence boundaries

Reviewed 2026-11-07. These are primary sources and implementation observations used to design the guide. External results are not Colocad benchmark results. The guide's detailed workflows, tool fields and tradeoffs are project recommendations to test, not copied universal rules.

| Source | What it supports | Colocad design decision |
| --- | --- | --- |
| [Normal: CAD Arena](https://normal.ai/research/cadarena) | Engineering drawings to editable CAD parts as an evaluation target | Test the requested geometry and editability; do not equate a preview with a reconstructed product. The static page did not expose its results table during this review, so no ranking is quoted. |
| [DIP-CAD, arXiv 2609.22688v4](https://arxiv.org/abs/2609.22688v4) | References, sketch placement and dependencies affect design intent under edits | Use explicit datums/semantic references and verify a real parameter change, separately from initial shape validity. This does not claim this kernel implements DIP-CAD or inherits its reported gains. |
| [Parametric CAD Bench v3, arXiv 2611.02659v1](https://arxiv.org/abs/2611.02659v1) | Geometry, specification and modeling history require separate evaluation; scores describe complete agent systems | Separate creation, edit preservation and drawing reconstruction; retain inspectable artifacts. No leaderboard score is attributed to Colocad. |
| [Anthropic: Writing effective tools for agents](https://www.anthropic.com/research/writing-tools-for-agents) | Tool boundaries, informative responses and realistic evaluations matter | Keep semantic operations, preserve schemas/errors, and provide a compact receipt with paths to full evidence. The adapter is tested against actual local native calls. |
| [Anthropic: Effective context engineering](https://www.anthropic.com/research/effective-context-engineering-for-ai-agents) | Relevant context can be loaded on demand and task state kept outside the immediate prompt | Load selected views/references; persist source IDs, accepted revision, unknowns and the next check. Do not repeat complete meshes or traces in every turn. |
| [Claude Platform: Skill authoring best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices) | Concision, progressive disclosure, appropriate freedom and usage-based validation | A short skill entry point routes to task-specific references, executable helpers and filled assets. Prompt examples are adaptable, not mandatory approvals. |
| [HIWIN: KK-series catalog](https://hiwin.com/wp-content/uploads/2026/08/HIWIN-Ballscrew-Driven-Stages-Catalog-1.pdf) | Dimensions/configurations for the selected real product | Source interfaces to the catalog and explicitly label approximate internals. The file hash/pages and assumptions are in the example's sources.json. |

## What was actually executed

- KK60: native construction and assembly across 16 milestones, 52 final features and 81 named solids; independent OCCT STEP checks, material probes and selected rail/carriage clearance checks. [Evidence](../showcase/hiwin-kk60/verification.json).
- A complete KK60 build in Linux arm64, 1 vCPU / 512 MiB, network disabled, with preview. One run; no peak-memory sampling or equivalent build123d run for this product. [Execution scope](../showcase/hiwin-kk60/evidence.json).
- SDK/session/error recovery, Pi integration, offline viewer controls/framing, CLI MP4 export, and installed-package checks. The new compact tool adapter and guided workflow execute real native geometry and quantitative edits; their decisions are scripted.
- The skill's structure/references and packaged examples are validated. No cross-model blind reconstruction study or skill-prompt ablation is claimed.

A native geometry pass, a harness integration pass, a successful agent run and a controlled skill improvement are distinct kinds of evidence. The handbook keeps those distinctions explicit.
