#!/usr/bin/env python3
"""Supplemental saved-file token counts, not provider usage or generation cost."""
import argparse
import json
from pathlib import Path
import tiktoken
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('files',type=Path,nargs='+');p.add_argument('--encoding',default='o200k_base')
a=p.parse_args();encoder=tiktoken.get_encoding(a.encoding)
for f in a.files:
    content=f.read_text()
    print(json.dumps({'file':str(f),'encoding':a.encoding,'tokens':len(encoder.encode(content,disallowed_special=())),'bytes':len(content.encode()),'measurement':'saved_file_only'}))
