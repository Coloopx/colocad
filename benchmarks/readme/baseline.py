"""Independent build123d implementation of the features used by the two fixtures."""
import json
import sys
import time
from pathlib import Path

start = time.perf_counter()
import build123d as bd
loaded = time.perf_counter()

tree = json.loads(Path(sys.argv[1]).read_text())
params = tree['parameters']
def val(x):
    return params[x['param']] if isinstance(x, dict) else x

body, seeds = None, {}
for f in tree['features']:
    op = f['op']
    if op in ('extrude', 'cut'):
        p = f['profile']
        if p['kind'] == 'circle':
            sketch = bd.Circle(val(p['radius']))
        elif p['kind'] == 'rectangle':
            sketch = bd.Rectangle(*map(val, p['size']))
        elif p['kind'] == 'polygon':
            sketch = bd.Polygon(*(tuple(map(val, xy)) for xy in p['points']), align=None)
        else:
            raise ValueError(p['kind'])
        x, y = map(val, p.get('center', (0, 0)))
        tool = bd.extrude(sketch, amount=val(f['depth'])).translate((x, y, val(f.get('startZ', 0))))
        seeds[f['id']] = (op, tool)
        body = tool if body is None else body.cut(tool) if op == 'cut' else body.fuse(tool)
    elif op == 'fillet':
        z = val(f['edges']['z'])
        edges = [e for e in body.edges() if max(abs(a.Z-z) for a in (e.bounding_box().min, e.bounding_box().max)) < 0.001]
        assert len(edges) == val(f['edges']['count'])
        body = bd.fillet(edges, val(f['radius']))
    elif op == 'linearPattern':
        seed_op, tool = seeds[f['seed']]
        pitch = val(f['pitch'])
        copies = [tool.translate((i*pitch, 0, 0) if f['axis'] == 'x' else (0, i*pitch, 0))
                  for i in range(1, val(f['count']))]
        body = body.cut(*copies) if seed_op == 'cut' else body.fuse(*copies)
    else:
        raise ValueError(op)

built = time.perf_counter()
assert body.is_valid and len(body.solids()) == 1 and body.volume > 0
assert bd.export_step(body, sys.argv[2])
print(json.dumps({'import_ms': (loaded-start)*1000, 'geometry_ms': (built-loaded)*1000}))
