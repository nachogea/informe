# Ship a useful native viewer

**Implementation plan · Viewer pilot**

Deliver a document experience that feels natural for reports, dashboards, and plans. Reading quality is the acceptance gate; review features remain available through secondary controls.

## 1. Establish the reading surface

- [x] Replace stacked controls with one compact toolbar.
- [x] Keep developer tools and review outlines out of normal reading.
- [x] Support Markdown files directly through the native renderer.
- [ ] Validate the experience with outside users.

## 2. Make documents easy to explore

People should be able to find a section, inspect a value, and copy a useful passage without learning a component selection mode.

1. Use **Contents** to navigate sections.
2. Use **Command-F** to find matching blocks; Enter advances and Shift-Enter goes back.
3. Hover chart rows for values and share of total.
4. Use column arrows to sort table data.

## 3. Keep the launch loop dependable

Validate content before opening it. Reuse the existing native window and tab, preserve a valid report during malformed edits, and acknowledge only after its native content is parsed.

```sh
informe --check examples/viewer/dashboard.json
informe open examples/viewer/dashboard.json
informe open examples/viewer/plan.md
```

## 4. Measure before making claims

Record cold and warm opens and polling reloads on representative documents. Report median and tail observations together with fixture size, sample count, machine, and readiness boundaries.

> A native renderer is an implementation choice. A consistently good reading experience is the product promise.

## Acceptance checklist

- [ ] A narrative report remains readable at narrow and wide sizes.
- [ ] A dashboard makes comparisons easy to understand.
- [ ] A plan displays headings, lists, links, and code clearly.
- [ ] Scrolling and resizing remain usable on longer documents.
- [ ] Installation succeeds on a clean Mac.

## After the viewer pilot

Signed distribution, a Homebrew cask, searchable PDF exports, and broader agent setup follow the viewer acceptance pass. Keep new features tied to observed user needs.

For project context, see the [repository](https://github.com/nachogea/informe). Return to the [acceptance checklist](#acceptance-checklist) when evaluating this pilot.
