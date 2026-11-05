"""Create language-specific GitHub scorecards directly from all measured rows."""
import html,json,math,statistics
from pathlib import Path
root=Path(__file__).resolve().parents[2]
dirs=root/'docs/benchmarks';src=[dirs/'runs.json',root/'showcase/tendon-hand/evidence/scope.json']
rows=sum((json.loads(p.read_text())['runs'] for p in src),[])
keys=['sensor-rail','manifold','finger','hand']; stats={}
for k in keys:
    stats[k]={}
    for backend in ('colocad','build123d'):
        r=[v for v in rows if v['case']==k and v['backend']==backend and v['rep']>0]
        assert len(r)==5 and all(v['oom_kill']==0 for v in r)
        stats[k][backend]=[statistics.median(v['wall_ms'] for v in r),max(v['peak_bytes'] for v in r)/2**20]
    v,w=stats[k].values();stats[k]['speedup']=w[0]/v[0];stats[k]['memory_fraction']=v[1]/w[1]
(dirs/'scorecard.json').write_text(json.dumps(stats,indent=2)+'\n')
for zh in [False,True]:
 def L(en,cn):return cn if zh else en
 def tx(x,y,msg,size=18,bold=False,fill='#193f43'):
    return f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{700 if bold else 400}" fill="{fill}">{html.escape(str(msg))}</text>'
 top='#087969'; base='#566d81'
 g=['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1100 625" role="img" aria-labelledby="title desc"><title id="title">Time to STEP / peak memory</title><desc id="desc">Median wall time and maximum cgroup peak for five fresh builds per backend per object. Absolute values in every row.</desc><rect width="1100" height="625" rx="14" fill="#f6f8f5"/><g font-family="system-ui, Arial, sans-serif">']
 g+=[tx(28,38,L('01  /  LOCAL CAD TOOL CALL','01  /  沙箱内一次 CAD 工具调用'),16,True,top),
     tx(28,86,L('How long until an agent can open the STEP?','从启动工具到拿到 STEP，要等多久？'),32,True),
     tx(28,121,L('Linux arm64   ·   1 vCPU / 512 MiB   ·   5 measured runs per backend and part','Linux arm64   ·   1 vCPU / 512 MiB   ·   每个零件、每个后端各测 5 次'),18)]
 for x,c,t in [(765,top,'colocad'),(906,base,'build123d')]:
    g.extend([f'<rect x="{x}" y="22" width="15" height="15" rx="3" fill="{c}"/>',tx(x+22,36,t,17,True,c)])
 g+= [tx(28,172,L('Workload','任务'),18,True),tx(252,172,L('Time to STEP  /  median','交付 STEP 用时  /  中位数'),18,True),tx(695,172,L('Peak sandbox memory  /  max','沙箱峰值内存  /  最高值'),18,True)]
 time_scale=math.ceil(max(stats[k][b][0] for k in keys for b in ('colocad','build123d'))/1000)*1000
 for i,k in enumerate(keys):
    y=200+96*i
    g.append(f'<line x1="28" y1="{y-12}" x2="1072" y2="{y-12}" stroke="#c4d2ce"/>')
    g+= [tx(28,y+22,L(['Sensor rail','Port manifold','Proximal phalanx','Complete hand'][i],['传感器导轨','四盲孔歧管','近节指骨','整手装配'][i]),21,True),tx(28,y+49,'43 '+L('solids','个实体') if i==3 else '1 '+L('solid','个实体'),15,False,'#55706c')]
    for j,(backend,co) in enumerate([('colocad',top),('build123d',base)]):
      t,m=stats[k][backend];yy=y+6+j*27
      for xx,length,val in [(252,t/time_scale*275,f'{t:,.0f} ms'),(695,m/512*265,f'{m:.1f} MiB')]:
        g+= [f'<rect x="{xx}" y="{yy}" width="{length:.3f}" height="13" rx="2" fill="{co}"/>',tx(xx+length+9,yy+13,val,16,True,co)]
 g.append(tx(28,600,L('Clock includes process startup, build, checks and STEP export. Memory = cgroup peak, not the configured limit.','时间含进程启动、建模、检查和 STEP 导出。内存是容器实测峰值，非配置上限。'),16,False,'#49666a'))
 (dirs/('time-to-step'+('.zh-CN' if zh else '')+'.svg')).write_text(''.join(g)+'</g></svg>\n')
