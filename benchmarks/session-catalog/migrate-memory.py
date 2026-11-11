"""Measure a single SDK importer in a fresh Linux cgroup. Each case gets a new project."""
from contextlib import closing
import argparse
import hashlib
import json
import os
import re
import sqlite3
import sys
import shutil
import subprocess
import tempfile
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('fixture', type=Path, help='directory of count/session.json fixtures')
p.add_argument('output', type=Path, help='new directory for this measurement and trace')
p.add_argument('binary', type=Path, help='Linux arm64 colocad-catalog to run')
p.add_argument('count', type=int)
p.add_argument('--limit-mib', type=int, default=512)
p.add_argument('--image', default='colocad-memory:20261111')
p.add_argument('--prepare', choices=('copy', 'clone'), default='copy',
               help='clone uses a fresh inode with shared storage blocks (both sides must use the same method)')
p.add_argument('--gate-mib', type=int, help='require peak at/below threshold, no limit hits and no OOM')
a = p.parse_args()
repo = Path(__file__).resolve().parents[2]
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=False)
n = str(a.count)
limit = str(a.limit_mib)+'m'


def sha(path):
    with path.open('rb') as f:
        digest = hashlib.sha256()
        for block in iter(lambda: f.read(1 << 20), b''):
            digest.update(block)
        return digest.hexdigest()


with tempfile.TemporaryDirectory(dir=out) as tmp:
    cmd = ['docker', 'run', '--rm', '--platform', 'linux/arm64', '--network', 'none',
           '--cpus', '1', '--memory', limit, '--memory-swap', limit,
           '-v', f'{tmp}:/project']
    fixture = a.fixture.resolve() / n / 'session.json'
    size = fixture.stat().st_size
    # Reserve room for the input if copied, growing DB/indexes, journal and host services.
    if shutil.disk_usage(tmp).free < (size if a.prepare=='copy' else 0) + 1.3*size + (128 << 20):
        raise RuntimeError('insufficient disk headroom for this fixture')
    if a.prepare == 'clone':
        clone_flag = '-c' if sys.platform == 'darwin' else '--reflink=always'
        subprocess.run(['cp', clone_flag, str(fixture), str(Path(tmp)/'session.json')], check=True)
    else:
        subprocess.run([*cmd, '-v', f'{a.fixture.resolve()}:/fixtures:ro', a.image,
                        'cp', f'/fixtures/{n}/session.json', '/project/session.json'], check=True)
    subprocess.run([*cmd, '-v', f'{out}:/out', '-v', f'{repo}:/pkg:ro',
                    '-v', f'{a.binary.resolve()}:/pkg/kernel/native/linux-arm64/colocad-catalog:ro',
                    a.image, 'python', '/pkg/benchmarks/memory/monitor.py',
                    'node', '/pkg/benchmarks/session-catalog/workload.mjs',
                    'migrate', n, '/project'], check=True)
    row = json.loads((out/'measurement.json').read_text())
    if row['status'] != 'ok':
        raise RuntimeError(row['status'])
    root = Path(tmp)
    name = json.loads((root/'session.json').read_text())['catalog']
    if not re.fullmatch(r'session-[0-9a-f-]{36}\.sqlite', name):
        raise RuntimeError('unexpected catalog descriptor')
    catalog = root / name
    backup = root / (catalog.name + '.v1.json')
    with closing(sqlite3.connect(catalog.as_uri()+'?mode=ro', uri=True)) as db:
        assert db.execute('pragma integrity_check').fetchone() == ('ok',)
        assert db.execute('select count(*),min(seq),max(seq) from steps').fetchone() == (a.count,1,a.count)
        assert db.execute("select id from steps where status='succeeded' and operation not in ('observe','query') order by seq desc limit 1").fetchone() == (f's{a.count:07d}',)
    source_hash = sha(a.fixture / n / 'session.json')
    assert sha(backup) == source_hash
    (out/'verified.json').write_text(json.dumps(dict(entries=a.count, catalog_bytes=catalog.stat().st_size,
        prepare=a.prepare, backup_sha256=source_hash, helper_sha256=sha(a.binary), integrity='ok'), indent=2)+'\n')
    if a.gate_mib and (row['peak_bytes'] > a.gate_mib * 2**20 or any(row['events'][k] for k in ('max','oom','oom_kill'))):
        raise RuntimeError('migration memory gate failed; inspect measurement.json')
    # Only this temporary, now verified DB is disposable. Some VM shares keep
    # host descriptors alive after unlink, consuming disk across repetitions.
    fd = os.open(catalog, os.O_WRONLY | os.O_NOFOLLOW)
    try:
        if os.fstat(fd).st_nlink == 1:
            os.ftruncate(fd, 0)
    finally:
        os.close(fd)
