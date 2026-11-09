"""Reproduce SDD 0004's outward-interface/legacy-mate contract conflict.

This exercises the existing native executable, not the unimplemented query API.
No CadQuery runtime or additional Python packages are required. Build artifacts
are isolated in a temporary directory and removed after extracting evidence.
"""

import argparse
import copy
import hashlib
import json
import platform
import subprocess
import tempfile
from pathlib import Path


def run(native):
    model = {
        "version": 1,
        "units": "mm",
        "parameters": {},
        "features": [
            {"id": "block", "op": "box", "origin": [-2, -2, 0], "size": [4, 4, 4]}
        ],
        "assembly": [
            {"id": "a", "input": "block"},
            {"id": "b", "input": "block", "translation": [0, 0, 4]},
        ],
    }
    # The top of A touches the bottom of B; both surface frames point outward.
    a = {"instance": "a", "frame": {"origin": [0, 0, 4], "x_axis": [1, 0, 0], "y_axis": [0, 1, 0]}}
    b = {"instance": "b", "frame": {"origin": [0, 0, 0], "x_axis": [1, 0, 0], "y_axis": [0, -1, 0]}}
    plane = {"id": "surface", "kind": "plane", "a": a, "b": b, "distance": 0, "opposed": True}
    concentric = {"id": "axis", "kind": "concentric", "a": a, "b": b}
    fixed = {"id": "fixed", "kind": "fixed", "a": a, "b": b}
    cases = [
        ("opposed_plane", [plane], ["a", "b"], 0, True),
        ("opposed_plane_plus_concentric", [plane, concentric], ["a", "b"], 0, False),
        ("opposed_fixed", [fixed], ["a", "b"], 0, False),
        ("plane_free_sliding", [plane], ["a"], 3, True),
    ]
    results = []
    with tempfile.TemporaryDirectory(prefix="colocad-spec-review-") as temp:
        for name, constraints, anchors, dof, expected_ok in cases:
            source = copy.deepcopy(model)
            source["mates"] = {
                "anchors": anchors,
                "expected_degrees_of_freedom": dof,
                "constraints": constraints,
            }
            process = subprocess.run(
                [str(native), "build", "--out", str(Path(temp) / name)],
                input=json.dumps(source), capture_output=True, text=True, timeout=30,
            )
            receipt = json.loads(process.stdout)
            assert receipt["ok"] is expected_ok, (name, receipt)
            assert process.returncode == (0 if expected_ok else 1), (name, process.returncode)
            item = {"case": name, "ok": receipt["ok"]}
            if expected_ok:
                diagnostic = receipt["data"]["assembly"]["mate_solution"]
                assert diagnostic["degrees_of_freedom"] == dof
            else:
                item["error_code"] = receipt["error"]["code"]
                assert item["error_code"] == "MATE_INCONSISTENT"
                diagnostic = json.loads(receipt["error"]["message"])
            item.update({
                "degrees_of_freedom": diagnostic["degrees_of_freedom"],
                "max_residual": diagnostic["max_residual"],
                "constraint_residuals": diagnostic["constraints"],
            })
            results.append(item)
    return {
        "scope": "Existing native mate semantics; not a new query implementation or a CadQuery runtime comparison",
        "platform": {"system": platform.system(), "machine": platform.machine()},
        "native_sha256": hashlib.sha256(native.read_bytes()).hexdigest(),
        "results": results,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(run(args.native.resolve()), indent=2))
