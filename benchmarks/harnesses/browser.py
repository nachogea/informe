#!/usr/bin/env python3
"""Headed Chromium cold-process launch to fonts + two animation frames."""
import argparse
import functools
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import threading
import time
from playwright.sync_api import sync_playwright

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--artifact',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--trials',type=int,default=5);p.add_argument('--environment',required=True)
    p.add_argument('--trial-offset',type=int,default=0)
    a=p.parse_args()
    if a.trials<1:p.error('positive trials required')
    artifact=a.artifact.resolve()
    if not artifact.name.endswith(('.standalone.html','.shared.html')):p.error('expected paired HTML fixture')
    class Handler(SimpleHTTPRequestHandler):
        def log_message(self,*args):pass
    server=ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Handler,directory=str(artifact.parent)))
    threading.Thread(target=server.serve_forever,daemon=True).start()
    a.output.parent.mkdir(parents=True,exist_ok=True)
    try:
        with sync_playwright() as pw, a.output.open('a') as out:
            for trial in range(a.trial_offset,a.trial_offset+a.trials):
                browser=None
                r=dict(task=artifact.name.split('.')[0],format='html_shared' if '.shared.' in artifact.name else 'html_standalone',model='none-fixed-fixture',environment=a.environment,scenario='cold-process',success=False,trial=trial,output_bytes=artifact.stat().st_size,marker='html_fonts_two_raf')
                try:
                    start=time.monotonic();browser=pw.chromium.launch(headless=False)
                    page=browser.new_page(viewport={'width':1100,'height':760})
                    page.goto(f'http://127.0.0.1:{server.server_port}/{artifact.name}')
                    page.wait_for_function('window.artifactReady !== undefined',polling='raf',timeout=30000)
                    r.update(open_ms=(time.monotonic()-start)*1000,success=True,browser=browser.version)
                    # Outside the timing interval: verify semantic selection and copy.
                    page.context.grant_permissions(['clipboard-read','clipboard-write'])
                    target=page.locator('[data-node-id]').last;target.click()
                    expected=target.get_attribute('data-text')
                    page.locator('#copy').click()
                    page.wait_for_function('(text) => navigator.clipboard.readText().then(x => x === text)',arg=expected)
                    r['interaction_check']=True
                    page.screenshot(path=str(a.output.parent/f'{artifact.stem}-{trial}.png'),full_page=True)
                except Exception as e:r.update(success=False,error=str(e))
                finally:
                    if browser:browser.close()
                out.write(json.dumps(r)+'\n');out.flush()
    finally:server.shutdown();server.server_close()
if __name__=='__main__':main()
