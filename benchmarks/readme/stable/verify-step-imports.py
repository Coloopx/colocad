"""Round-trip neutral files through a released native kernel and compare with OCCT."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer


def shape_metrics(path):
    reader = STEPControl_Reader()
    if int(reader.ReadFile(str(path))) != 1 or not reader.TransferRoots():
        raise ValueError(f'OCCT could not read {path.name}')
    shape = reader.OneShape()
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-10)
    bounds = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, bounds, False, False)
    solids = TopExp_Explorer(shape, TopAbs_SOLID)
    count = 0
    while solids.More():
        count += 1
        solids.Next()
    return dict(n=count, valid=BRepCheck_Analyzer(shape).IsValid(),
                volume=props.Mass(), bbox=list(bounds.Get()))


def command(binary, argv, payload):
    proc = subprocess.run([binary, *map(str, argv)], input=json.dumps(payload).encode(),
                          capture_output=True, timeout=70)
    return json.loads(proc.stdout)


def source(path):
    content = path.read_bytes()
    return {'sha256': hashlib.sha256(content).hexdigest(), 'step': content.decode()}


def failure(reply):
    err = reply['error']
    return err['code'] + ' ' + err['message'][:280]


def roundtrip(binary, path, out):
    asset = source(path)
    info = command(binary, ['inspect-step'], asset)
    row = {'sha256': asset['sha256'], 'inspect': 'ok' if info['ok'] else failure(info)}
    if not info['ok']:
        return row
    row.update(definitions=info['data']['definitions'], instances=len(info['data']['instances']))
    model = info['data']['model_template']
    model['assets'] = {'source': asset}
    built = command(binary, ['build', '--out', out], model)
    row['build'] = 'ok' if built['ok'] else failure(built)
    if built['ok']:
        a, b = shape_metrics(path), shape_metrics(out/'model.step')
        row.update(source=a, delivered=b)
        row['match'] = (a['valid'] and b['valid'] and a['n'] == b['n']
                        and max(abs(x-y) for x,y in zip(a['bbox'], b['bbox'])) < 1e-4
                        and abs(a['volume']-b['volume']) < max(1e-4, 1e-6*abs(a['volume'])))
    return row


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('binary', type=Path)
parser.add_argument('output', type=Path)
parser.add_argument('--appearance-runs', type=Path, help='Five-product runs/ directory with case/cadquery/r1/model.step')
args = parser.parse_args()
repo = Path(__file__).resolve().parents[3]
binary = args.binary.resolve()
inputs = {}
for directory in ('specs/experiments/0004/fixtures/engineering','crates/agent-cad/tests/fixtures'):
    for path in sorted((repo/directory).glob('*.step')):
        assert path.stem not in inputs
        inputs[path.stem] = path
if args.appearance_runs:
    products = sorted(args.appearance_runs.glob('*/cadquery/r1/model.step'))
    assert len(products) == 5
    inputs.update({'appearance-'+p.parents[2].name: p for p in products})
with tempfile.TemporaryDirectory() as temp:
    root = Path(temp)
    rows = {name: roundtrip(binary, path, root/name) for name, path in inputs.items()}
    results = {'native_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'external_step': rows}
    block = source(repo/'crates/agent-cad/tests/fixtures/occt-drilled-block.step')
    features = [dict(id=f'd{i}',op='import_step',asset='source',body_index=0,definition=True) for i in range(32)]
    instances = [dict(id=f'i{i}',input=f['id'],translation=[32*i,0,0]) for i,f in enumerate(features)]
    model = dict(version=1,units='mm',parameters={},assets={'source':block},features=features,assembly=instances)
    for name, limits in [('allowed',{}),('small-input',{'stepBytes':1024}),
                         ('small-output',{'outputStepBytes':1024}),('small-report',{'reportBytes':1024})]:
        out = root/name
        reply = command(binary,['build','--out',out,'--resource-limits',json.dumps(limits)], model)
        if limits:
            assert not reply['ok'] and not out.exists() and reply['error']['code']=='RESOURCE_LIMIT'
            details = reply['error']['details']
            assert details['resource']==next(iter(limits))
            results[name]=details
        else:
            assert reply['ok']
            cache = reply['data']['step_source_cache']
            assert (cache['parse_calls'],cache['hits'],cache['retained_models'])==(1,31,0)
            geom = shape_metrics(out/'model.step')
            assert geom['valid'] and geom['n']==32
            results[name] = {'cache':cache, 'occt':geom}
args.output.write_text(json.dumps(results,indent=2)+'\n')
print(f"{sum(r.get('match') is True for r in rows.values())}/{len(rows)} STEP round trips passed")
