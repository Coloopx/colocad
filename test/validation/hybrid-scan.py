"""Generate the external stress input from Stanford's unmodified Armadillo PLY."""
from pathlib import Path
import gzip
import hashlib
import json
import struct
import sys
import urllib.request

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
url = "https://graphics.stanford.edu/pub/3Dscanrep/armadillo/Armadillo.ply.gz"
raw = urllib.request.urlopen(url, timeout=60).read()
assert hashlib.sha256(raw).hexdigest() == "8b9b56cc36e66d54429b1e1e75bd89e833645bfe0dc7c1afd1205877a7356a3f"
header, data = gzip.decompress(raw).split(b"end_header\n", 1)
n_vertices = 172974
assert b"element vertex 172974" in header and b"element face 345944" in header
start = struct.unpack_from(">fff", data)
with (out / "armadillo.obj").open("w") as obj:
    for point in struct.iter_unpack(">fff", data[:12*n_vertices]):
        obj.write("v %s %s %s\n" % point)
    for _, n, a, b, c in struct.iter_unpack(">BBiii", data[12*n_vertices:]):
        assert n == 3
        obj.write(f"f {a+1} {b+1} {c+1}\n")
spec = {
    "format": "colocad/hybrid/1", "units": "mm", "facetedStep": True,
    "title": "Stanford Armadillo and alignment probes",
    "features": [
        {"id": "scan", "op": "import_mesh", "path": "armadillo.obj", "units": "mm"},
        {"id": "datum", "op": "box", "min": [-1,-1,-1], "max": [1,1,1]},
    ],
    "instances": [
        {"id": "armadillo", "input": "scan"},
        {"id": "datum", "input": "datum"},
        {"id": "surface-probe", "input": "datum", "translate": start},
    ],
    "checks": [
        {"a": "armadillo", "b": "datum"},
        {"a": "armadillo", "b": "surface-probe", "maxOverlapMm3": 8},
    ],
}
(out / "model.json").write_text(json.dumps(spec))
