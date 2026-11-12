"""Build the candidate comparison and offline report after the trials and their readbacks finish."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

p=argparse.ArgumentParser()
p.add_argument('trial',type=Path);p.add_argument('destination',type=Path)
args=p.parse_args();trial=args.trial.resolve();dest=args.destination.resolve();dest.mkdir(parents=True,exist_ok=True)
repo=Path(__file__).resolve().parents[3]
def read(path):return json.loads(path.read_text())
def save(path,value):path.write_text(json.dumps(value,indent=2)+'\n')
def run(script,*args):subprocess.run([sys.executable,repo/script,*args],check=True)

campaign=read(trial/'campaign.json')
assert campaign['label']=='0.1.0-alpha.11-rc.1'
# The binary/source identity is independent from the recipe used to test it.
campaign['workload_sources']={name:hashlib.sha256((repo/name).read_bytes()).hexdigest() for name in [
    'benchmarks/memory/plan-v1.json','benchmarks/memory/workload.py','benchmarks/memory/workload.mjs',
    'benchmarks/release/fixture.py','showcase/tendon-hand/bench/window.py','benchmarks/field-task/orders.json',
    'benchmarks/field-task/colocad.mjs','benchmarks/field-task/python-backends.py','benchmarks/field-task/python-flange.py',
    'benchmarks/field-task/verify-comparison.py','examples/engineering-interfaces/verify.py']}
save(trial/'campaign.json',campaign)
for mode,count in [('main',108),('fixtures',99)]:
    assert len(read(trial/(mode+'-verified.json')))==count
for script,params in [('report.py',[trial,dest]),('scaling.py',[dest]),('unique-phases.py',[trial,dest]),
                      ('race.py',[trial,dest]),('embed.py',[dest]),('shape-gallery.py',[trial/'shapes',dest])]:
    run('benchmarks/release/'+script,*params)
for file in ['main-verified.json','fixtures-verified.json','windows-verified.json','kk60-readback.json']:
    shutil.copyfile(trial/file,dest/file)
cover=read(trial/'cover.gif.json');cover.pop('file',None)
assert cover['frames']==144 and not cover['browserErrors']
save(dest/'kk60-render.json',cover)
for ext in ['gif','mp4']:
    shutil.copyfile(trial/('cover.'+ext),repo/'showcase/hiwin-kk60'/('modeling-alpha11-rc1.'+ext))
corpus={}
for split in ['g2-regression','g2-held_out']:
    report=read(trial/split/'results.json');assert report['passed']
    corpus[split]={k:report[k] for k in ['split','manifest_sha256','source_commit','native_sha256','families','passed']}
    corpus[split]['count']=len(report['rows'])
save(dest/'geometry-corpus.json',corpus)
for script,params in [('engineering.py',[trial,dest]),('engineering-gif.py',[trial,dest/'engineering-changes.gif']),
                      ('memory-gif.py',[dest]),('extend-offline.py',[dest])]:
    run('benchmarks/readme/candidate/'+script,*params)
