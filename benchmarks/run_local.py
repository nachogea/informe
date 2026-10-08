#!/usr/bin/env python3
"""Interleave renderer pilot trials; no inference calls or generation claims."""
import argparse
import json
from pathlib import Path
import random
import subprocess
import sys
ROOT=Path(__file__).resolve().parent
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
p.add_argument('--environment',required=True);p.add_argument('--trials',type=int,default=5);p.add_argument('--seed',type=int,default=42)
a=p.parse_args()
if a.trials<1:p.error('positive trials required')
if a.output.exists():p.error('choose a new output file; existing results are retained')
jobs=[(task,arm,trial) for task in ['dashboard','cost-breakdown','long-review'] for arm in ['native','standalone','shared'] for trial in range(a.trials)]
random.Random(a.seed).shuffle(jobs)
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.with_suffix('.schedule.json').write_text(json.dumps({'seed':a.seed,'jobs':jobs},indent=2)+'\n')
for task,arm,trial in jobs:
    cmd=[sys.executable,str(ROOT/'harnesses'/('native.py' if arm=='native' else 'browser.py')),'--artifact',str(ROOT/'fixtures'/(task+('.json' if arm=='native' else f'.{arm}.html'))),'--output',str(a.output.resolve()),'--environment',a.environment,'--trials','1','--trial-offset',str(trial)]
    if arm=='native':cmd+=['--binary',str(a.binary.resolve())]
    print(f'{task}: {arm} trial {trial}',flush=True)
    subprocess.run(cmd,check=True)
