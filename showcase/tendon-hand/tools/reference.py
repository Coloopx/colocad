"""Reference preparation only. Never an input to the CAD builder."""
import argparse
from pathlib import Path
import mujoco as mj
import numpy as np
from projection import CAMERAS,render,contact
p=argparse.ArgumentParser();p.add_argument('source',type=Path);p.add_argument('dest',type=Path);a=p.parse_args()
m=mj.MjModel.from_xml_path(str(a.source/'right_hand.xml'));d=mj.MjData(m);mj.mj_forward(m,d)
palm=m.body('rh_palm').id;meshes=[]
for g in range(m.ngeom):
    k=m.geom_dataid[g]
    if m.geom_group[g]!=2 or k<0 or m.body(m.geom_bodyid[g]).name=='rh_forearm':continue
    v=m.mesh_vert[m.mesh_vertadr[k]:m.mesh_vertadr[k]+m.mesh_vertnum[k]]
    f=m.mesh_face[m.mesh_faceadr[k]:m.mesh_faceadr[k]+m.mesh_facenum[k]]
    world=v@d.geom_xmat[g].reshape(3,3).T+d.geom_xpos[g]
    local=(world-d.xpos[palm])@d.xmat[palm].reshape(3,3)*1000
    meshes.append((local,f,[155,168,174]))
a.dest.mkdir(exist_ok=True,parents=True)
views={k:render(meshes,cam) for k,cam in CAMERAS.items()}
for k,v in views.items():v.save(a.dest/(k+'.png'))
contact(views,'01   /   Shadow Hand E3M5     ·     Reference views').save(a.dest/'four-views.png')
