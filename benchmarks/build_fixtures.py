#!/usr/bin/env python3
"""Build fixed, offline renderer fixtures; these are not agent generation trials."""
from pathlib import Path
import html
import json

ROOT = Path(__file__).resolve().parent
CSS = '''*{box-sizing:border-box}body{margin:0;font:14px -apple-system,BlinkMacSystemFont,sans-serif;color:#202a40;background:#f5f7fb}header{padding:20px 24px;background:white;border-bottom:1px solid #e1e6ef;font-weight:700}.layout{display:flex;height:calc(100vh - 59px)}main{flex:1;overflow:auto;padding:32px;min-width:440px}aside{width:260px;flex-shrink:0;padding:24px;background:white;border-left:1px solid #e1e6ef}.node{border:1px solid transparent}.selected{border-color:#5468da}.column,.card{display:flex;flex-direction:column;align-items:flex-start;gap:16px}.column{width:100%}.row{display:flex;gap:12px;width:100%}.row>.node{flex:1;min-width:180px}.card,.metric,.chart,.table{background:white;border-radius:10px;padding:20px;border-color:#e1e6ef;width:100%}h1,h2,h3,h4,h5,h6,p{margin:0}h1{font-size:30px}h2{font-size:24px}h3{font-size:20px}p,.label{color:#6c7890}.value{font-size:30px;font-weight:700;margin:8px 0}.secondary{color:#5468da}.callout{padding:16px;border-radius:8px;background:#edf2ff;width:100%}.warning{background:#fff6e5}.error{background:#ffeeee}.success{background:#edf9f1}.callout p{margin-top:8px}.button,button{background:#5468da;color:white;padding:10px 16px;border-radius:6px;border:0;cursor:pointer}.divider{width:100%;border-top:1px solid #e1e6ef}.bars{margin-top:16px}.bar{display:flex;align-items:center;gap:12px;margin:12px 0}.bar-label{width:105px}.track{flex:1;background:#eef1f7;height:18px;border-radius:4px}.fill{height:100%;background:#5468da;border-radius:4px}.bar-value{width:84px;text-align:right}.axis{display:flex;justify-content:space-between;color:#6c7890}table{width:100%;border-collapse:collapse;margin-top:16px;table-layout:fixed}th,td{text-align:left;padding:10px;border-bottom:1px solid #e1e6ef;overflow-wrap:anywhere}tbody tr:nth-child(even){background:#f5f7fb}pre{white-space:pre-wrap;font:inherit}'''
JS = '''document.addEventListener('click',async e=>{if(e.target.id==='copy'){await navigator.clipboard.writeText(window.selected?.dataset.text||'');return}const n=e.target.closest('[data-node-id]');if(!n)return;document.querySelector('.selected')?.classList.remove('selected');n.classList.add('selected');window.selected=n;document.querySelector('#selection').textContent='ID\\n'+n.dataset.nodeId+'\\n\\nTYPE\\n'+n.dataset.type;document.querySelector('#copy').hidden=false});document.fonts.ready.then(()=>requestAnimationFrame(()=>requestAnimationFrame(()=>{window.artifactReady={marker:'html_fonts_two_raf',nodes:document.querySelectorAll('[data-node-id]').length}})));'''
def esc(value): return html.escape(str(value), quote=True)
def text(n):
    k=n['type']
    if k in ('column','row','card'): return '\n\n'.join(filter(None,(text(c) for c in n['children'])))
    if k in ('heading','text'): return n['text']
    if k=='metric': return '\n'.join(n[x] for x in ('label','value','secondary') if n.get(x) is not None)
    if k=='callout': return '\n'.join(n[x] for x in ('title','body') if n.get(x) is not None)
    if k=='button': return n['label']
    if k=='chart': return n['title']+'\n'+'\n'.join(d['label']+'\t'+str(d['value'])+((' '+n['unit']) if n.get('unit') else '') for d in n['data'])
    if k=='table': return '\n'.join('\t'.join(c.replace('\t',' ').replace('\n',' ').replace('\r',' ') for c in row) for row in [n['columns'],*n['rows']])
    return ''
def render(n):
    k=n['type']; attrs=f'class="node {k} {esc(n.get("kind",""))}" data-node-id="{esc(n["id"])}" data-type="{k}" data-text="{esc(text(n))}"'
    if k in ('column','row','card'):
        inner=''.join(render(c) for c in n['children']); attrs+=f' style="gap:{n.get("gap") if n.get("gap") is not None else (12 if k=="row" else 16)}px"'
    elif k=='heading': inner=f'<h{n["level"]}>{esc(n["text"])}</h{n["level"]}>'
    elif k=='text': inner=f'<p>{esc(n["text"])}</p>'
    elif k=='metric': inner=f'<div class="label">{esc(n["label"])}</div><div class="value">{esc(n["value"])}</div><div class="secondary">{esc(n.get("secondary") or "")}</div>'
    elif k=='callout': inner=(f'<strong>{esc(n["title"])}</strong>' if n.get('title') else '')+f'<p>{esc(n["body"])}</p>'
    elif k=='button': inner=esc(n['label'])
    elif k=='divider': inner=''
    elif k=='chart':
        maximum=max(d['value'] for d in n['data']); inner=f'<strong>{esc(n["title"])}</strong><div class="bars">'
        for d in n['data']:
            width=100*d['value']/maximum if maximum else 0
            inner+=f'<div class="bar"><span class="bar-label">{esc(d["label"])}</span><div class="track"><div class="fill" style="width:{width}%"></div></div><span class="bar-value">{esc(d["value"])} {esc(n.get("unit") or "")}</span></div>'
        inner+=f'</div><div class="axis"><span>0</span><span>{maximum}</span></div>'
    elif k=='table':
        inner=f'<strong>{esc(n["title"])}</strong><table><thead><tr>'+''.join(f'<th>{esc(c)}</th>' for c in n['columns'])+'</tr></thead><tbody>'
        inner+=''.join('<tr>'+''.join(f'<td>{esc(c)}</td>' for c in row)+'</tr>' for row in n['rows'])+'</tbody></table>'
        if not n['rows']: inner+='<p>No rows</p>'
    else: raise ValueError(k)
    return f'<div {attrs}>{inner}</div>'
def main():
    fixtures=ROOT/'fixtures'; fixtures.mkdir(exist_ok=True)
    (fixtures/'theme.css').write_text(CSS+'\n'); (fixtures/'interaction.js').write_text(JS+'\n')
    long={'version':1,'root':{'type':'column','id':'review','children':[{'type':'heading','id':'review-title','level':1,'text':'Native artifact pilot review'},{'type':'text','id':'review-description','text':'Illustrative project review: repeated sections test scrolling and semantic selection.'}]+[{'type':'card','id':f'phase-{i}','children':[{'type':'heading','id':f'phase-{i}-title','level':2,'text':f'Phase {i}: Review milestone'},{'type':'text','id':f'phase-{i}-body','text':'Validate semantic content, preserve stable IDs, and measure the review loop.'},{'type':'callout','id':f'phase-{i}-risk','kind':'info','title':'Next check','body':'Verify the renderer and interaction contract before expanding the node vocabulary.'}]} for i in range(1,13)]}}
    sources={'dashboard':json.loads((ROOT.parent/'examples/dashboard.json').read_text()),'cost-breakdown':json.loads((ROOT.parent/'artifacts/cost-breakdown.json').read_text()),'long-review':long}
    for name,artifact in sources.items():
        (fixtures/f'{name}.json').write_text(json.dumps(artifact,indent=2)+'\n')
        body='<header>Artifact Preview</header><div class="layout"><main>'+render(artifact['root'])+'</main><aside><strong>Selected node</strong><pre id="selection">Click an artifact component.</pre><button id="copy" hidden>Copy text</button></aside></div>'
        for arm in ('standalone','shared'):
            assets=f'<style>{CSS}</style>' if arm=='standalone' else '<link rel="stylesheet" href="theme.css">'
            script=f'<script>{JS}</script>' if arm=='standalone' else '<script src="interaction.js"></script>'
            (fixtures/f'{name}.{arm}.html').write_text(f'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>{name}</title>{assets}<body>{body}{script}</body></html>\n')
        brief={'task':name,'facts':artifact,'quality_requirements':['All supplied facts; readable at 1100x760 with scrolling.','Stable semantic IDs, deepest-node selection, selected-node copy; noop buttons.','Use supplied facts for all three arms; no invented functional actions.'],'edits':['Change a metric value if present; preserve its ID.','Append a table row if present; preserve rectangular shape.','Rewrite a callout body; preserve its ID.']}
        (ROOT/'tasks'/f'{name}.json').write_text(json.dumps(brief,indent=2)+'\n')
if __name__=='__main__': main()
