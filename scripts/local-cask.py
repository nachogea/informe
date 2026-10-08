#!/usr/bin/env python3
"""Generate a local cask from an authenticated download or local build."""
import argparse
import hashlib
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('archive',type=Path);p.add_argument('--version',required=True);p.add_argument('--output',type=Path,required=True)
a=p.parse_args();archive=a.archive.resolve()
if not a.version.replace('.','').replace('-','').isalnum():p.error('invalid version')
sha=hashlib.sha256(archive.read_bytes()).hexdigest()
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(f'''cask "nachogea-informe" do
  version "{a.version}"
  sha256 "{sha}"
  url "{archive.as_uri()}"
  name "Informe"
  desc "Native semantic artifact viewer powered by GPUI"
  homepage "https://github.com/nachogea/informe"
  depends_on arch: :arm64
  depends_on macos: :tahoe
  app "Informe.app"
  binary "#{{appdir}}/Informe.app/Contents/MacOS/informe"
end
''')
print(a.output)
