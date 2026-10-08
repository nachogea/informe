#!/usr/bin/env python3
"""Measure CLI handoff to a valid frame in an already-running document workspace."""
import argparse
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path, nargs='+', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--environment', required=True)
    parser.add_argument('--trials', type=int, default=20)
    args = parser.parse_args()
    if args.trials < 1:
        parser.error('positive trials required')
    binary = args.binary.resolve()
    paths = [path.resolve() for path in args.artifacts]
    # Preload each document. Setup/initial parsing is excluded from repeat-open timing.
    for path in paths:
        subprocess.run([str(binary), 'open', str(path)], check=True, capture_output=True, timeout=30)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('a') as output:
        for trial in range(args.trials):
            for path in paths:
                record = dict(task=path.stem, format='native_json' if path.suffix == '.json' else 'native_markdown',
                              model='none-fixed-fixture', environment=args.environment,
                              scenario='warm-repeat-open', trial=trial, success=False,
                              output_bytes=path.stat().st_size, marker='gpui_after_frame')
                start = time.monotonic()
                try:
                    result = subprocess.run([str(binary), 'open', str(path)], capture_output=True, timeout=30)
                    record['success'] = result.returncode == 0
                    record['open_ms'] = (time.monotonic() - start) * 1000
                    if not record['success']:
                        record['error'] = result.stderr.decode(errors='replace')[-2000:]
                except (OSError, subprocess.TimeoutExpired) as error:
                    record['error'] = str(error)
                output.write(json.dumps(record) + '\n')
                output.flush()


if __name__ == '__main__':
    main()
