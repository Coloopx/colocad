# Memory study evidence

See the [English report](../../memory-persistence-report.md), [中文报告](../../memory-persistence-report.zh-CN.md), and [single-file HTML](memory-study.html).

- `memory-study.json` / `.csv`: all 200 planned observations, 194 executed; failure categories remain visible. Unrun rows have no measured peaks.
- `environment.json`: actual runtime and dependency versions. `memory-study.json` retains each campaign's image/source/plan fingerprints.
- `verification.json`: 36 representative STEP deliveries independently reopened and measured. `catalog-verification.json`: all 15 chunked projects' complete file hashes, coverage and database integrity.
- `verification-manifest-diagnostic.json`: preserved postprocessing path/WAL diagnostics. These did not change the measured geometry or successful-run count.
- `raw-evidence.tar.gz`: original per-run cgroup measurements, 20 ms traces, events, stdout/stderr, receipts, container status, plans and source. `raw-manifest.json` contains file SHA-256 digests. It excludes the large STEP outputs and synthetic metadata fixtures; these can be regenerated from the source and parameters. Verification receipts retain selected output hashes.

The archive's `runner-snapshots/` contains the exact recorded runner versions, named by SHA-256. `source/` contains final experiment/analysis programs; original campaign manifests include hashes of incidental postprocessing programs that evolved while later controls were added. Baseline modeling and monitoring programs remained frozen. Follow the [reproduction procedure](../../../benchmarks/memory/README.md), including the baseline binary hash and pinned environment, when collecting a new campaign.

The HTML embeds all plots, table rows and scripts. It requires no CDN or external assets. The production SDK and native binary remain alpha.8; indexed/chunked experiments are prototypes for the [planned alpha.9 work](../../../specs/0006-bounded-memory-persistence.md).
