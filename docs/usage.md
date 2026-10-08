[← Documentation](README.md)

# Using the viewer

Open a UTF-8 Markdown file (`.md` or `.markdown`) or semantic JSON document:

```sh
informe open /absolute/path/report.md
```

`open` validates the file, opens or focuses its tab in the running app, and waits
for a valid rendered frame. **Open…** and Command-O bring up the native file picker.
The app also lists recent documents when all tabs are closed. Finder document
associations and drag-and-drop opening are not implemented.

## Read and navigate

Use **Contents** to jump to a heading, chart, or table. **Find** (Command-F)
searches plain text, including table titles. Matches are document blocks, not
individual occurrences. Escape dismisses Find or the More menu.

Reports and plans have a narrower reading column. Dashboard rows stack at narrow
window widths, and wide tables scroll horizontally. Click a table column header
to sort; repeated clicks reverse its direction. A column sorts numerically when
all nonblank values parse as numbers, otherwise as case-insensitive text. Blank
cells stay last. Sorting leaves the source document untouched.

## Copy and export

Drag or double-click text, then use Command-C. Command-A selects the entire
document; Command-C then includes content below the viewport. **More → Copy
document text** provides the same whole-document output without a selection.

Tables have **Copy TSV** and **Save CSV…** controls. Both preserve source row
order, quote cells as needed, and omit the table title from the rectangular data.
Whole-document text includes the title. Copied plans preserve `[x]` and `[ ]`.

Optional **More → Copy image (PNG)** and **Export PNG…** include offscreen content
without app chrome. Headless export is also available:

```sh
informe export-png /absolute/path/report.md /tmp/report.png
```

PNG uses a separate 960-point-wide layout. Markdown styling and Markdown tables
flatten to text; chart number formatting can differ from the window. Long
reports use a lower image scale and exports above 20,000 points are rejected.
Color emoji may not render correctly. For a faithful picture of the visible
window, use a macOS screenshot. PDF export is not implemented.

## Live updates and errors

The viewer checks for changes every 250 ms. A valid update replaces the document
and clears text selection. An invalid update displays an error while retaining
the last valid document. Fix the file and the viewer recovers automatically.
Agents should validate a sibling temporary file and rename it to the preview
path; see [the agent workflow](agent-workflow.md).

## Local data

Documents remain at the paths you choose. Recent document paths and the local
handoff socket live under `~/Library/Application Support/Informe`.
On first launch after upgrading from Artifact Preview, Informe reads the old
recent-file list if it has not saved its own. Existing document files and their
review sidecars stay at their original paths.
Normal reading does not create review sidecars. Experimental review stores
comments and snapshots beside the document; see [review storage](review.md#storage-and-provenance).
The app has no built-in model API or cloud synchronization. Links open in your
system's configured app when clicked.

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| `informe: command not found` | Install the CLI and ensure `~/.local/bin` is on the PATH inherited by your shell and agent. |
| A fresh source build is slow | Cargo downloads and compiles the pinned dependencies on first build. Later launches use the built executable. |
| `open` reports validation errors | Run `informe --check PATH`; check the file extension and the [format reference](artifact-format.md). |
| An edit does not appear | Look for the error banner and validate the file at the exact path shown in the tab. |
| The app appears to be an older build | Quit the viewer with Command-Q, reinstall/rebuild, then reopen. An existing process keeps its original code and environment. |
| An open request times out | Check the running viewer, quit it normally, and retry with the absolute document path. The handoff waits up to 20 seconds. |
| An unsigned downloaded bundle will not launch | Signed downloads are not available yet. Use the supported source-build workflow; the packaging pilot is not an end-user installer. |

For unresolved problems, [open an issue](https://github.com/nachogea/informe/issues)
with the version, macOS version, architecture, reproduction steps, and a small
synthetic document. Remove private report data before sharing it.
