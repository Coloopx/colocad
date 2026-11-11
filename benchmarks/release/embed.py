"""Finalize the offline report after report.py and race.py."""
import base64,json,sys
from pathlib import Path
d=Path(sys.argv[1]);p=d/'scorecard.html';s=p.read_text()
assert 'id="rawdata"' not in s,'Run report.py first'
s=s.replace('<section><table><thead>','<section><table id="measurements"><thead>').replace('querySelectorAll("tbody tr")','querySelectorAll("#measurements tbody tr")')
gif=base64.b64encode((d/'hand-window.gif').read_bytes()).decode()
s=s.replace('<h2>All 23 comparable tasks',f'<figure><img src="data:image/gif;base64,{gif}" width="100%" alt="10× replay of the three measured hand windows, with running peak memory"></figure><h2>All 23 comparable tasks')
e=json.loads((d/'observations.json').read_text())
payload=json.dumps(e,separators=(',',':')).replace('<','\\u003c')
s=s.replace('</main>',f'<p><button id="download">Save the underlying observations and memory samples (JSON)</button></p><script type="application/json" id="rawdata">{payload}</script><script>document.querySelector("#download").onclick=()=>{{let url=URL.createObjectURL(new Blob([document.querySelector("#rawdata").textContent],{{type:"application/json"}})),a=document.createElement("a");a.href=url;a.download="colocad-{e["campaign"]["artifact_slug"]}-observations.json";a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}}</script></main>')
p.write_text(s)
