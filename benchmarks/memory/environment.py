"""Run inside the benchmark image after all timed campaigns."""
import json,platform,sqlite3,subprocess
code='import json,sys,importlib.metadata as m;print(json.dumps({"python":sys.version,"distributions":{d.metadata["Name"]:d.version for d in m.distributions()}}))'
print(json.dumps({'architecture':platform.machine(),'kernel':platform.release(),'node':subprocess.check_output(['node','-p','JSON.stringify(process.versions)'],text=True).strip(),'python':platform.python_version(),'catalog_sqlite':sqlite3.sqlite_version,'backends':{b:json.loads(subprocess.check_output([p,'-c',code],text=True)) for b,p in [('cadquery','/opt/cq/bin/python'),('build123d','/opt/bd/bin/python')]}},indent=2))
