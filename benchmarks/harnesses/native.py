#!/usr/bin/env python3
"""Measure native cold-process and polling reload frame-marker latency."""
import argparse
import json
from pathlib import Path
import platform
import subprocess
import tempfile
import time

def wait(marker,revision,process,timeout):
    start=time.monotonic()
    while time.monotonic()-start < timeout:
        if process.poll() is not None: raise RuntimeError(f'viewer exited: {process.returncode}')
        try:
            if json.loads(marker.read_text())['revision']>=revision: return
        except (OSError,ValueError,KeyError): pass
        time.sleep(.002)
    raise TimeoutError('content-ready marker timed out')
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,required=True);p.add_argument('--artifact',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True);p.add_argument('--trials',type=int,default=5)
    p.add_argument('--timeout',type=float,default=30);p.add_argument('--environment',required=True)
    p.add_argument('--scenario',choices=['cold-process','reload'],default='cold-process')
    p.add_argument('--trial-offset',type=int,default=0)
    a=p.parse_args()
    if a.trials<1 or a.timeout<=0:p.error('positive trials and timeout required')
    payload=a.artifact.read_bytes(); binary=a.binary.resolve()
    a.output.parent.mkdir(parents=True,exist_ok=True)
    with a.output.open('a') as out, tempfile.TemporaryDirectory() as folder:
        folder=Path(folder)
        for trial in range(a.trial_offset,a.trial_offset+a.trials):
            artifact=folder/('artifact'+a.artifact.suffix); marker=folder/'ready.json';marker.unlink(missing_ok=True);artifact.write_bytes(payload)
            record=dict(task=a.artifact.stem,format='native_markdown' if a.artifact.suffix.lower() in ('.md','.markdown') else 'native_json',model='none-fixed-fixture',environment=a.environment,scenario=a.scenario,success=False,trial=trial,output_bytes=len(payload),marker='gpui_after_frame',timing_poll_ms=2)
            process=None
            try:
                start=time.monotonic();process=subprocess.Popen([str(binary),'--ready-file',str(marker),str(artifact)],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
                wait(marker,1,process,a.timeout)
                if a.scenario=='reload':
                    # A valid semantic edit, not just whitespace. Content is also
                    # validated by the runtime before the revision marker advances.
                    if a.artifact.suffix.lower() in ('.md','.markdown'):
                        edited=payload.decode('utf-8')+'\n\nReload measurement edit.\n'
                    else:
                        data=json.loads(payload);data['root']['id']+='-reload';edited=json.dumps(data)
                    pending=artifact.with_suffix('.pending');pending.write_text(edited)
                    pending.replace(artifact);start=time.monotonic()
                    wait(marker,2,process,a.timeout)
                record['reload_ms' if a.scenario=='reload' else 'open_ms']=(time.monotonic()-start)*1000
                record['success']=True
            except (OSError,RuntimeError,TimeoutError,ValueError) as e:record['error']=str(e)
            finally:
                if process is not None:
                    process.terminate()
                    try:_,err=process.communicate(timeout=5)
                    except subprocess.TimeoutExpired:process.kill();_,err=process.communicate()
                    if err:record['stderr']=err.decode(errors='replace')[-2000:]
            out.write(json.dumps(record)+'\n');out.flush()
if __name__=='__main__':main()
