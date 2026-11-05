"""Read the separately timed finger and hand exports. The result images use actual STEP geometry."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

import build123d as bd
import numpy as np
from projection import render, CAMERAS
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'bench'))
from verify import vol, difference

def meshes(parts, color=(95,162,157)):
    result=[]
    for part in parts:
        v,i=part.tessellate(.12,.20)
        result.append((np.asarray([tuple(x) for x in v]),np.asarray(i),color))
    return result

def save_thumbnail(image, path):
    from PIL import Image
    # Crop to the CAD silhouette before fitting it to the time track.
    pixels=np.asarray(image)
    mask=pixels[:,:,0]<220
    ys,xs=np.where(mask)
    cropped=image.crop((max(xs.min()-9,0),max(ys.min()-9,0),min(xs.max()+10,image.width),min(ys.max()+10,image.height)))
    cropped.thumbnail((227,103),Image.Resampling.LANCZOS)
    tile=Image.new('RGB',(232,105),'#f5f6f2')
    tile.paste(cropped,((232-cropped.width)//2,(105-cropped.height)//2));tile.save(path)

def prepare(trial, dest):
    trace=json.loads((trial/'trace.json').read_text());case=trace['case'];stats={};properties=[]
    for row in trace['recorded']:
        arm=row['backend'];path=trial/f'{case}-{arm}.step'
        assert hashlib.sha256(path.read_bytes()).hexdigest()==row['step_sha256']
        shape=bd.import_step(path)
        parts=shape.children or [shape]
        count=len(parts)
        assert shape.is_valid and len(shape.solids())==count=={'finger':1,'hand':43}[case]
        properties.append([([*p.bounding_box().min,*p.bounding_box().max],vol(p.wrapped)) for p in parts])
        stats[arm]={'valid':True,'solids':count,'faces':len(shape.faces()),'edges':len(shape.edges()),'sha256':row['step_sha256']}
        if case=='hand':
            picture=render(meshes(parts),CAMERAS['oblique'])
        else:
            picture=render(meshes(parts),(1,-.9,.56),5.35,np.asarray([33.,0.,117.5]),(430,380))
        picture.save(dest/f'{case}-{arm}-large.png')
        save_thumbnail(picture,dest/f'{case}-{arm}.png')
    stats['max_bounds_delta_mm']=max(abs(x-y) for (bb,_),(bc,_) in zip(*properties,strict=True) for x,y in zip(bb,bc))
    stats['max_relative_volume_delta']=max(abs(a-b)/b for (_,a),(_,b) in zip(*properties,strict=True))
    assert stats['max_bounds_delta_mm']<1e-4 and stats['max_relative_volume_delta']<1e-6
    if case=='finger':
        first,second=[bd.import_step(trial/f'finger-{name}.step').wrapped for name in ('colocad','build123d')]
        stats['difference_mm3']=[difference(first,second),difference(second,first)]
        assert max(stats['difference_mm3'])<1e-4
    (dest/f'{case}-readback.json').write_text(json.dumps(stats,indent=2)+'\n')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('trial',type=Path);p.add_argument('output',type=Path)
    args=p.parse_args();args.output.mkdir(exist_ok=True,parents=True);prepare(args.trial,args.output)
