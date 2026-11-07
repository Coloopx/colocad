# Agent guidance and release completion

The user requested a complete material-to-model guide, skill/prompt design, tool abstraction, workflow and tuning guidance, while preserving their README capitalization change and completing npm/Release delivery.

Acceptance:
- G1: the handbook separates observed implementation from recommendations and external research; it maps inputs, outputs, uncertainty, interfaces, edits, verification and delivery.
- G2: the skill is concise and routes to relevant references; materials/brief metadata are not claimed to be executable kernel input. Prompt examples preserve the user's selected product and authorization scope.
- G3: compact tool responses preserve schemas, exact measurements, error/degradation details and full-receipt paths. A real native test checks recovery from a failed edit.
- G4: a runnable guided example performs actual capability/validation/build/edit/observe calls, checks numerical changes and exports HTML through the CLI. Scripted decisions are explicitly distinguished from live model inference.
- G5: English/Chinese README explain SDK/CLI/skill and show the interactive viewer; the user's `Colocad` capitalization is retained.
- G6: publication uses an isolated credential configuration and checks expected identity and archive integrity. A previously published version with a different archive must not be overwritten. Existing alpha.4 Git tags remain immutable; new guidance ships in alpha.5.

Verification: `test/tool-summary.test.mjs`, `test/publish-config.test.mjs`, skill quick validation, the guided example, installed tarball examples, full Node/integration/browser regressions, and public registry/release integrity checks. No cross-model blind study or prompt-improvement percentage is claimed.
