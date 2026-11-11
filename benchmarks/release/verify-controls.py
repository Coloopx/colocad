"""Independent readback of all delivered monoliths and the sampled archived shards."""
import argparse
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
parser.add_argument("mode", choices=("monoliths", "shards"))
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
out = args.output.resolve()
image = "colocad-memory:20261111"

def readback(manifest, destination):
    subprocess.run([
        "docker", "run", "--rm", "--network", "none", "--cpuset-cpus", "5,7", "--cpus", "2",
        "--memory", "7g", "--memory-swap", "7g", "-v", f"{repo}:/repo:ro",
        "-v", f"{out}:/data", image, "/opt/cq/bin/python", "/repo/benchmarks/memory/verify.py",
        "/data/" + str(manifest.relative_to(out)), "/data/" + str(destination.relative_to(out))
    ], check=True)

if args.mode == "shards":
    subprocess.run(["python3", repo / "benchmarks/memory/verification-manifest.py", out], check=True)
    rows = json.loads((out / "verification-manifest.json").read_text())
    path = out / "shard-samples.json"
    path.write_text(json.dumps([r for r in rows if r["id"].startswith("sharded/")]))
    readback(path, out / "shard-samples-verified.json")
else:
    dest = out / "aux-readback"
    dest.mkdir(exist_ok=True)
    cases = []
    for suite in ("boundary", "compositional", "native-control", "extreme"):
        runs = json.loads((out / suite / "runs.json").read_text())["runs"]
        for row in runs:
            if row["status"] == "ok":
                step, = (out / suite / row["id"]).rglob("model.step")
                cases.append(dict(id=suite + "/" + row["id"], path="/data/" + str(step.relative_to(out)),
                                  family=row["family"], n=row["n"]))
    for case in sorted(cases, key=lambda row: (row["n"], row["family"])):
        stem = case["id"].replace("/", "-")
        manifest = dest / (stem + ".json")
        manifest.write_text(json.dumps([case]))
        readback(manifest, dest / (stem + "-verified.json"))
