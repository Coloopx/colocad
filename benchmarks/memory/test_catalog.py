import json,sqlite3,subprocess,sys,tempfile,unittest
from pathlib import Path
from catalog import Catalog
class Durability(unittest.TestCase):
    def test_reopen_and_ranges(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'project.sqlite';c=Catalog(path)
            c.append_chunk(0,32,{'path':'chunks/0/model.step'});c.append_chunk(32,7,{'path':'chunks/32/model.step'});c.close();c=Catalog(path)
            self.assertEqual(c.locate(38)[:2],(32,7))
            for i in [-1,39]:
                with self.assertRaises(KeyError):c.locate(i)
            with self.assertRaises(ValueError):c.append_chunk(40,1,{})
            self.assertEqual(c.locate(38)[:2],(32,7));c.close()
    def test_process_crash_does_not_commit_partial_transaction(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'project.sqlite';c=Catalog(p);c.append_event(1,{'status':'accepted'});c.close()
            code='import sqlite3,os,sys;c=sqlite3.connect(sys.argv[1]);c.execute("BEGIN");c.execute("INSERT INTO events VALUES(2,?)",(\'{}\',));os._exit(19)'
            self.assertEqual(subprocess.run([sys.executable,'-c',code,str(p)]).returncode,19)
            c=Catalog(p);self.assertEqual(c.latest(),(1,{'status':'accepted'}));c.append_event(2,{'status':'accepted'});c.close()
            c=Catalog(p);self.assertEqual(c.latest()[0],2);self.assertEqual(c.db.execute('PRAGMA integrity_check').fetchone()[0],'ok');c.close()
    def test_bounded_index_lookup_plan(self):
        with tempfile.TemporaryDirectory() as d:
            c=Catalog(Path(d)/'project.sqlite')
            for i in range(1000):c.append_chunk(i*32,32,{'path':str(i)})
            plan=c.db.execute('EXPLAIN QUERY PLAN SELECT record FROM chunks WHERE start<=? ORDER BY start DESC LIMIT 1',(31000,)).fetchall()
            self.assertIn('SEARCH',str(plan));self.assertEqual(c.db.execute('PRAGMA cache_size').fetchone()[0],-2048);self.assertEqual(c.locate(31999)[0],31968);c.close()
if __name__=='__main__':unittest.main()
