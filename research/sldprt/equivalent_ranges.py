"""Prove range redundancy per carrier before expressing the same trimming by its vertices."""
import json, math, sys
from pathlib import Path
ir=json.loads(Path(sys.argv[1]).read_text());m=ir['model']
index={e['id']:e for key in ['curves','vertices','points'] for e in m[key]}
xyz=lambda p:[p[k] for k in 'xyz']
def vertex(j):return xyz(index[index[j]['point']]['position'])
def eq(a,b):return math.dist(a,b)<1e-7
for edge in m['edges']:
 e=edge['carrier'];bounds=e.get('param_range')
 if bounds is None:continue
 g=index[e['curve']]['geometry'];a,b=vertex(edge['start']),vertex(edge['end'])
 if g['kind']=='line':
  o=xyz(g['origin']);v=xyz(g['direction'])
  def at(t):return [x+t*y for x,y in zip(o,v)]
  assert eq(at(bounds[0]),a) and eq(at(bounds[1]),b) and bounds[1]>bounds[0]
 elif g['kind']=='circle':
  assert edge['start']==edge['end'] and eq(bounds,[0,math.tau])
  assert eq(a,[x+g['radius']*y for x,y in zip(xyz(g['center']),xyz(g['ref_direction']))])
 elif g['kind']=='nurbs':
  d=g['degree'];k=g['knots'];p=g['poles']['points']
  assert not g['periodic'] and k[:d+1]==[k[0]]*(d+1) and k[-d-1:]==[k[-1]]*(d+1)
  assert bounds==[k[0],k[-1]] and eq(a,xyz(p[0])) and eq(b,xyz(p[-1]))
 else:raise ValueError(g['kind'])
 del e['param_range']
Path(sys.argv[2]).write_text(json.dumps(ir))
