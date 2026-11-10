import json,sys
from pathlib import Path
from catalog import Catalog
n=int(sys.argv[1]);c=Catalog('/history/'+str(n)+'/project.sqlite');latest=c.latest();assert latest[0]==n
assert latest[1]['id']=='s'+str(n).zfill(7);c.close()
Path('/out/result.json').write_text(json.dumps({'ok':True,'entries':n,'latest':latest[1]['id'],'variant':'experimental indexed catalog read','synthetic_metadata':True})+'\n')
