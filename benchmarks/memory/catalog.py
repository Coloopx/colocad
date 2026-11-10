"""Experimental bounded-cache catalog. NOT the published ModelingSession format."""
import hashlib,json,sqlite3
from pathlib import Path

def digest(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()

class Catalog:
    def __init__(self,path):
        self.db=sqlite3.connect(path)
        self.db.executescript('''PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
        PRAGMA cache_size=-2048; PRAGMA mmap_size=0; PRAGMA temp_store=FILE;
        CREATE TABLE IF NOT EXISTS chunks(start INTEGER PRIMARY KEY, count INTEGER NOT NULL, record TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY, record TEXT NOT NULL);''')
    def append_chunk(self,start,count,record):
        if count<1:raise ValueError('empty chunk')
        with self.db:
            last=self.db.execute('SELECT start,count FROM chunks ORDER BY start DESC LIMIT 1').fetchone()
            if start!=(sum(last) if last else 0):raise ValueError('noncontiguous chunk')
            self.db.execute('INSERT INTO chunks VALUES(?,?,?)',(start,count,json.dumps(record)))
    def locate(self,index):
        row=self.db.execute('SELECT start,count,record FROM chunks WHERE start<=? ORDER BY start DESC LIMIT 1',(index,)).fetchone()
        if not row or index>=row[0]+row[1]:raise KeyError(index)
        return row[0],row[1],json.loads(row[2])
    def append_event(self,seq,record):
        with self.db:self.db.execute('INSERT INTO events VALUES(?,?)',(seq,json.dumps(record)))
    def latest(self):
        row=self.db.execute('SELECT seq,record FROM events ORDER BY seq DESC LIMIT 1').fetchone()
        return None if row is None else (row[0],json.loads(row[1]))
    def close(self):self.db.close()
