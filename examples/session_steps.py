"""Read closed Colocad projects for the independent Python geometry verifiers."""
from pathlib import Path
from contextlib import closing
import json
import re
import sqlite3


def session_steps(directory):
    root = Path(directory).resolve()
    with (root / 'session.json').open() as stream:
        manifest = json.load(stream)
    if manifest.get('format') == 'colocad/session/1':
        yield from manifest['steps']
        return
    name = manifest.get('catalog', '')
    if manifest.get('format') != 'colocad/session/2' or not re.fullmatch(r'session-[a-f0-9-]{36}\.sqlite', name):
        raise ValueError('Unsupported session descriptor')
    db = (root / name).resolve()
    if db.parent != root:
        raise ValueError('Catalog outside session')
    with closing(sqlite3.connect(db.as_uri() + '?mode=ro', uri=True)) as conn:
        if conn.execute('PRAGMA application_id').fetchone()[0] != 0x434f4c4f or conn.execute('PRAGMA user_version').fetchone()[0] != 1:
            raise ValueError('Unsupported session catalog')
        for entry, in conn.execute('SELECT entry FROM steps ORDER BY seq'):
            yield json.loads(entry)
