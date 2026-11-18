"""Measure two shipped kernels on identical mechanical inputs (macOS or GNU time)."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def workloads():
    compile_source = "import {kernelModel} from './src/tree.mjs';" \
        "import {readFileSync} from 'node:fs';" \
        "console.log(JSON.stringify(kernelModel(JSON.parse(readFileSync('examples/manifold.json')))))"
    manifold = json.loads(subprocess.check_output(
        ["node", "--input-type=module", "-e", compile_source], cwd=ROOT))
    output = manifold["output"]

    def box(name, x):
        return dict(id=name, op="box", origin=[x, 0, 0], size=[10, 10, 10])

    def occurrence(name, source, x):
        return dict(id=name, input=source, translation=[x, 0, 0])

    def assembly(features, instances, checks):
        return dict(version=1, units="mm", parameters=manifold["parameters"],
                    features=features, assembly=instances, checks=checks)

    def collision(instances):
        return dict(kind="collision", parts=[item["id"] for item in instances], max_overlap_mm3=0.001)

    yield "manifold", manifold
    head = [occurrence("body", "stock", 0), occurrence("cavity", "other", 0)]
    yield "collision-then-manifold", assembly(
        [box("stock", 0), box("other", 5)] + manifold["features"],
        head + [occurrence("last", output, 110)], [collision(head)])
    repeated = [occurrence(f"part{i}", output, 110*i) for i in range(24)]
    yield "24-separated-manifolds", assembly(manifold["features"], repeated, [collision(repeated)])
    yield "24-separated-no-checks", assembly(manifold["features"], repeated, [])
    fixture = ROOT / "specs/experiments/0004/fixtures/engineering/kk60-accessory-bracket.step"
    raw = fixture.read_bytes()
    for mode in ("ordinary", "definition"):
        part = dict(op="import_step", id="bracket", asset="source", body_index=0)
        if mode == "definition":
            part["definition"] = True
        yield f"kk60-{mode}", dict(version=1, units="mm", parameters={}, features=[part],
            output="bracket", assets={"source": dict(step=raw.decode(), sha256=hashlib.sha256(raw).hexdigest())})


def measure(binary, raw, directory, name):
    mac = sys.platform == "darwin"
    timer = ["-l"] if mac else ["-f", "%M maximum resident set size"]
    argv = ["/usr/bin/time", *timer, str(binary), "build", "--out", str(directory)]
    start = time.perf_counter()
    process = subprocess.run(argv, input=raw, capture_output=True, timeout=300)
    elapsed = (time.perf_counter() - start) * 1000
    rss = int(re.search(rb"(\d+)\s+maximum resident set size", process.stderr)[1])
    receipt = json.loads(process.stdout)
    if name == "collision-then-manifold":
        assert not receipt["ok"] and receipt["error"]["code"] == "ASSEMBLY_COLLISION", receipt
        assert "500" in receipt["error"]["message"], receipt
        geometry = {"overlap_mm3": 500}
    else:
        assert process.returncode == 0 and receipt["ok"], receipt
        data = receipt["data"]
        geometry = data["geometry"]
        assert geometry["solids"] == (24 if name.startswith("24-") else 1)
        if name == "24-separated-manifolds":
            checks = data["assembly"]["checks"]
            assert len(checks) == 276 and all(row["overlap_mm3"] == 0 for row in checks)
    return dict(ms=elapsed, rss_mib=rss/(1048576 if mac else 1024), geometry=geometry)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--trials", type=int, default=7)
    args = parser.parse_args()
    assert args.trials >= 1
    args.out.mkdir(parents=True, exist_ok=True)
    bins = [("base", args.base.resolve()), ("candidate", args.candidate.resolve())]
    rows = []
    for name, model in workloads():
        raw = json.dumps(model).encode()
        for trial in range(args.trials+1):
            for backend, binary in (reversed(bins) if trial % 2 else bins):
                with tempfile.TemporaryDirectory(dir=args.out) as tmp:
                    directory = Path(tmp)/"revision"
                    row = measure(binary, raw, directory, name)
                    if trial == 0 and (directory/"model.step").exists():
                        shutil.copyfile(directory/"model.step", args.out/f"bench-{backend}-{name}.step")
                rows.append(dict(case=name, trial=trial, backend=backend, **row))
        for backend, _ in bins:
            sample = [r for r in rows if r["case"] == name and r["backend"] == backend and r["trial"]]
            print(name, backend, *(round(statistics.median(r[key] for r in sample), 3)
                                    for key in ("ms", "rss_mib")), flush=True)
        (args.out/"native-samples.json").write_text(json.dumps(rows, indent=1)+"\n")


if __name__ == "__main__":
    main()
