"""Collect verified release reruns without copying large intermediate CAD files."""
import argparse
import hashlib
import json
import statistics
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("run", type=Path)
parser.add_argument("destination", type=Path)
args = parser.parse_args()
run, dest = args.run, args.destination
read = lambda path: json.loads(path.read_text())
identity = read(run / "campaign.json")
result = {"campaign": identity, "hybrid": {}, "frozen_geometry": {}}
mib = 2**20

for case in ("housing", "scan"):
    folder = run / ("hybrid-" + case)
    report = read(folder / "report.json")
    assert report["ok"] and report["assemblyStep"] == "assembly.step"
    lines = (run / ("hybrid-" + case + ".log")).read_text().splitlines()
    start, end, peak = map(int, lines[0].split())
    events = {k: int(v) for k, v in (line.split() for line in lines[1:])}
    assert events["oom_kill"] == 0
    result["hybrid"][case] = {
        "wall_s": (end - start) / 1e9, "peak_mib": peak / mib,
        "step_bytes": (folder / report["assemblyStep"]).stat().st_size,
        "checked_triangles": report["instances"], "checks": report["checks"],
        "inspections": report["inspections"], "memory_events": events,
    }
scan = result["hybrid"]["scan"]
assert scan["checked_triangles"][0]["triangles"] == 345944
assert scan["checks"][0]["overlapMm3"] == 0 < scan["checks"][1]["overlapMm3"] < 8
result["hybrid"]["housing"]["readback"] = read(run / "hybrid-occt.log")
assert result["hybrid"]["housing"]["readback"]["assembly.step"]["solids"] == 3

for k in ("g2-regression", "g2-held"):
    proof = read(run / k / "results.json")
    assert proof["passed"] and proof["native_sha256"] == identity["macos_arm64"]["agent-cad"]
    result["frozen_geometry"][k] = {
        "cases": len(proof["rows"]), "manifest_sha256": proof["manifest_sha256"],
        "families": proof["families"],
    }

cover = read(run / "cover.gif.json")
assert not cover["browserErrors"] and len(cover["steps"]) == 16
assert cover["sha256"] == hashlib.sha256((run / "cover.gif").read_bytes()).hexdigest()
cover["file"] = "showcase/hiwin-kk60/modeling-alpha10.gif"
(dest / "kk60-render.json").write_text(json.dumps(cover, indent=2) + "\n")
for name in ("kk60", "turbofan"):
    proof = read(run / (name + "-readback.json"))
    assert proof["passed"] == 16
    result[name] = {"reopened_revisions": proof["passed"]}
    if name == "kk60":
        (dest / "kk60-readback.json").write_text(json.dumps(proof, indent=2) + "\n")
result["replacement"] = read(run / "flange/occt-readback.json")
result["external_step"] = read(run / "kk60-interface/occt-readback.json")

cases = {}
for path in sorted((run / "pairs").glob("*/comparison.json")):
    row = read(path)
    key = row["family"] + "/" + str(row["n"])
    cases.setdefault(key, []).append({k: row[k] for k in
        ("rep", "variant", "seconds", "peak_mib", "input_sha256", "sha256", "timing_ms")})
verified = read(run / "pairs/occt.json")
assert len(verified) == len(cases) == 12 and all(row["passed"] for row in verified)
paired = {"cases": {}, "sessions": {}, "occt_readbacks": verified}
for key, trials in cases.items():
    assert len({row["input_sha256"] for row in trials}) == len({row["sha256"] for row in trials}) == 1
    entry = {"trials": trials}
    for variant in ("before", "after"):
        rows = [row for row in trials if row["variant"] == variant]
        assert sorted(row["rep"] for row in rows) == [0, 1, 2]
        entry[variant] = {"wall_s": statistics.median(r["seconds"] for r in rows),
                          "peak_mib": statistics.median(r["peak_mib"] for r in rows)}
    paired["cases"][key] = entry
for name, variant in [("alpha.9", "before"), ("alpha.10", "after")]:
    receipt = read(run / ("audit-" + variant + ".log"))
    measured = read(run / ("session-" + variant) / "measurement.json")
    assert measured["status"] == "ok" and receipt["ok"] and receipt["count"] == measured["result"]["verifiedInOrder"] == 10001
    files = [p for p in (run / ("project-" + variant)).rglob("*") if p.is_file()]
    paired["sessions"][name] = {"entries": receipt["count"], "wall_s": measured["wall_s"],
        "peak_mib": measured["peak_bytes"] / mib, "files": len(files),
        "retained_bytes": sum(p.stat().st_size for p in files), "readback": receipt}
diag = run / "timing-diagnostic.json"
if diag.exists():
    paired["excluded_timing_attempts"] = read(diag)
result["revision_comparison"] = paired
(dest / "acceptance.json").write_text(json.dumps(result, indent=2) + "\n")
