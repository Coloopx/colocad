"""Create self-contained readback figures for six macOS controls."""
import argparse
import base64
import hashlib
import io
import json
import sys
from pathlib import Path
import numpy as np

repo = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(repo / "showcase/tendon-hand/tools"))
from projection import axes, render

p = argparse.ArgumentParser()
p.add_argument("run", type=Path)
p.add_argument("dest", type=Path)
a = p.parse_args()
data = a.run / "data"
raw = json.loads((data / "benchmark.json").read_text())
geometry = json.loads((data / "geometry.json").read_text())
assets = json.loads((data / "viewer-assets.json").read_text())
titles = {"plate4":"Four-hole mounting plate","plate64":"64-hole plate","fillet":"Filleted block","revolve":"270° revolve","product":"Electronics enclosure","surface":"Edited Bézier surface"}
assert set(titles)==set(geometry)==set(assets)==set(raw["runs"])
gallery = {}
observations = []
camera = (.72,-1,.74)
size=(350,175)
for name,title in titles.items():
    g = geometry[name]
    assert g["geometry_pass"], name
    gallery[name] = {"title":title,"readback":g["backends"],"backends":{}}
    meshes = {}
    for backend in ("rust","build123d"):
        trials=raw["runs"][name][backend]
        assert len(trials)==5 and all(r["ok"] for r in trials)
        for r in trials:
            observations.append({"case":name,"backend":backend,"rep":r["repetition"],"wall_ms":r["wall_ms"],"peak_rss_mib":r["peak_rss_mib"]})
        meshes[backend]=[]
        for part in assets[name]["meshes"][backend]:
            v=np.frombuffer(base64.b64decode(part["vertices"]),dtype="<f4").reshape(-1,3).astype(float)
            tri=np.frombuffer(base64.b64decode(part["indices"]),dtype="<u4").reshape(-1,3)
            meshes[backend].append((v,tri,[int(c*255) for c in part["color"]]))
    all_points=np.concatenate([v for parts in meshes.values() for v,_,_ in parts])
    center=(all_points.max(0)+all_points.min(0))/2
    bounds=(all_points-center)@axes(camera)
    scale=.8*min(size[i]/np.ptp(bounds[:,i]) for i in (0,1))
    for backend,parts in meshes.items():
        image=render(parts,camera,scale=scale,center=center,size=size)
        buf=io.BytesIO()
        image.save(buf,format="PNG",optimize=True)
        gallery[name]["backends"][backend]={"png":base64.b64encode(buf.getvalue()).decode(),
                                           "metrics":raw["summary"][name][backend]}
    gallery[name]["acceptance"]={k:v for k,v in g.items() if k.endswith(("delta","_mm","_pass","_relative","_mm3"))}
evidence={"method":raw["method"],"build123d":raw["environment"]["build123d"],"native_sha256":raw["rust_binary_sha256"],"source_sha256":raw["source_sha256"],"observations":observations,"cases":gallery}
(a.dest/"shapes.json").write_text(json.dumps(evidence,separators=(",",":"))+"\n")
