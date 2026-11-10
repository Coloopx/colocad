"""Isolate native checkpoint/query artifact cost; same direct invocation in all arms."""
import json,math,subprocess,sys
from pathlib import Path
variant=sys.argv[1];args=['/pkg/kernel/native/linux-arm64/agent-cad','build','--out','/out/native']
if variant!='plain':args+=['--checkpoints']
if variant=='queryable':args+=['--queryable']
r=subprocess.run(args,input=Path('/bench/fixtures/unique64.json').read_text(),capture_output=True,text=True)
Path('/out/native.stdout').write_text(r.stdout);Path('/out/native.stderr').write_text(r.stderr)
receipt=json.loads(r.stdout)
if receipt['ok']:
    expected=sum(((20+i*.0001)*20-16*math.pi)*6 for i in range(64))
    assert math.isclose(receipt['data']['geometry']['volume_mm3'],expected,rel_tol=1e-7)
Path('/out/result.json').write_text(json.dumps(receipt)+'\n')
sys.exit(r.returncode)
