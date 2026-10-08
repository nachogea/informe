//! Full-content export from semantic nodes, independent of viewport and app chrome.
//! SVG is an internal rasterization surface, never an artifact contract.
use crate::artifact::{ArtifactNode, CalloutKind};
use resvg::{tiny_skia, usvg};
use std::sync::Arc;

pub fn table_data(node: &ArtifactNode, csv: bool) -> Result<String, String> {
    let ArtifactNode::Table { columns, rows, .. } = node else {
        return Err("Select a table first".into());
    };
    let separator = if csv { ',' } else { '\t' };
    Ok(std::iter::once(columns)
        .chain(rows)
        .map(|row| {
            row.iter()
                .map(|cell| {
                    // CSV and quoted TSV preserve commas, tabs, quotes and embedded newlines.
                    if cell.contains([separator, '"', '\r', '\n']) {
                        format!("\"{}\"", cell.replace('"', "\"\""))
                    } else {
                        cell.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(&separator.to_string())
        })
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n")
}

pub fn png(node: &ArtifactNode) -> Result<Vec<u8>, String> {
    let mut fonts = usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    // macOS LastResort claims every glyph and can replace an entire mixed-script
    // run with placeholders. Let real script fonts supply fallback glyphs.
    let placeholders: Vec<_> = fonts
        .faces()
        .filter(|face| {
            face.families
                .iter()
                .any(|(name, _)| name.contains("LastResort"))
        })
        .map(|face| face.id)
        .collect();
    for id in placeholders {
        fonts.remove_face(id);
    }
    fonts.set_sans_serif_family("Helvetica Neue");
    let mut painter = Painter {
        svg: String::new(),
        fonts: Arc::new(fonts),
    };
    let height = painter.node(node, 32., 32., 896.) + 64.;
    if !height.is_finite() || height > 20_000. {
        return Err("Document is too tall for PNG export (20,000 points maximum)".into());
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="960" height="{height}"><rect width="100%" height="100%" fill="#f5f7fb"/>{}</svg>"##,
        painter.svg
    );
    let options = usvg::Options {
        fontdb: painter.fonts,
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())?;
    // At most 40 million pixels; long reports use 1x rather than exhausting RAM.
    let scale = if height * 960. * 4. <= 40_000_000. {
        2.
    } else {
        1.
    };
    let mut pixels = tiny_skia::Pixmap::new((960. * scale) as u32, (height * scale).ceil() as u32)
        .ok_or("Could not allocate export image")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixels.as_mut(),
    );
    pixels.encode_png().map_err(|e| e.to_string())
}

fn xml(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
struct Painter {
    svg: String,
    fonts: Arc<usvg::fontdb::Database>,
}
impl Painter {
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let id = self.fonts.query(&usvg::fontdb::Query {
            families: &[usvg::fontdb::Family::SansSerif],
            weight: usvg::fontdb::Weight(if bold { 700 } else { 400 }),
            ..Default::default()
        });
        id.and_then(|id| {
            self.fonts.with_face_data(id, |bytes, index| {
                let face = rustybuzz::Face::from_slice(bytes, index)?;
                let mut buffer = rustybuzz::UnicodeBuffer::new();
                buffer.push_str(text);
                let glyphs = rustybuzz::shape(&face, &[], buffer);
                Some(
                    glyphs
                        .glyph_positions()
                        .iter()
                        .map(|g| g.x_advance as f32)
                        .sum::<f32>()
                        * size
                        / face.units_per_em() as f32,
                )
            })
        })
        .flatten()
        .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }
    fn lines(&self, text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            // Preserve whitespace and break long identifiers when needed.
            for word in paragraph.split_inclusive(char::is_whitespace) {
                if !line.is_empty() && self.width(&(line.clone() + word), size, bold) > width {
                    lines.push(std::mem::take(&mut line));
                }
                if self.width(word, size, bold) <= width {
                    line.push_str(word);
                } else {
                    for c in word.chars() {
                        if !line.is_empty()
                            && self.width(&(line.clone() + &c.to_string()), size, bold) > width
                        {
                            lines.push(std::mem::take(&mut line));
                        }
                        line.push(c);
                    }
                }
            }
            lines.push(line);
        }
        lines
    }
    // Explicit typography and geometry keep the small export painter straightforward.
    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        bold: bool,
        color: &str,
    ) -> f32 {
        let lines = self.lines(text, width.max(1.), size, bold);
        for (i, line) in lines.iter().enumerate() {
            self.svg.push_str(&format!(r#"<text x="{x}" y="{}" font-family="Helvetica Neue, sans-serif" font-size="{size}" font-weight="{}" fill="{color}">{}</text>"#, y + size + i as f32 * size * 1.4, if bold {700} else {400}, xml(line)));
        }
        lines.len() as f32 * size * 1.4
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: &str, border: bool) {
        self.svg.push_str(&format!(r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="8" fill="{color}" stroke="{}"/>"##, if border {"#e1e6ef"} else {color}));
    }
    fn node(&mut self, node: &ArtifactNode, x: f32, y: f32, w: f32) -> f32 {
        use ArtifactNode::*;
        match node {
            Column { children, .. } | Card { children, .. } => {
                let card = matches!(node, Card { .. });
                let padding = if card { 24. } else { 0. };
                let gap = match node {
                    Column { gap, .. } => gap.unwrap_or(16.),
                    _ => 16.,
                };
                let start = self.svg.len();
                let mut height = padding;
                for (index, child) in children.iter().enumerate() {
                    if index > 0 {
                        height += gap;
                    }
                    height += self.node(child, x + padding, y + height, w - padding * 2.);
                }
                height += padding;
                if card {
                    let contents = self.svg.split_off(start);
                    self.rect(x, y, w, height, "#ffffff", true);
                    self.svg.push_str(&contents);
                }
                height
            }
            Row { gap, children, .. } => {
                let gap = gap.unwrap_or(12.);
                let cw = (w - gap * children.len().saturating_sub(1) as f32)
                    / children.len().max(1) as f32;
                let mut height: f32 = 0.;
                for (i, child) in children.iter().enumerate() {
                    height = height.max(self.node(child, x + i as f32 * (cw + gap), y, cw.max(1.)));
                }
                height
            }
            Heading { level, text, .. } => self.text(
                text,
                x,
                y,
                w,
                match level {
                    1 => 32.,
                    2 => 25.,
                    3 => 21.,
                    4 => 18.,
                    5 => 16.,
                    _ => 14.,
                },
                true,
                "#202a40",
            ),
            Markdown { .. } => self.text(&node.text_content(), x, y, w, 15., false, "#3a4658"),
            Text { text, .. } => self.text(text, x, y, w, 15., false, "#6c7890"),
            Divider { .. } => {
                self.rect(x, y + 8., w, 1., "#e1e6ef", false);
                17.
            }
            Metric {
                label,
                value,
                secondary,
                ..
            } => {
                let start = self.svg.len();
                let mut h = 24.;
                h += self.text(label, x + 24., y + h, w - 48., 13., false, "#6c7890") + 12.;
                h += self.text(value, x + 24., y + h, w - 48., 34., true, "#202a40");
                if let Some(s) = secondary {
                    h += 12.;
                    h += self.text(s, x + 24., y + h, w - 48., 12., false, "#5468da");
                }
                h += 24.;
                let contents = self.svg.split_off(start);
                self.rect(x, y, w, h, "#ffffff", true);
                self.svg.push_str(&contents);
                h
            }
            Callout {
                kind, title, body, ..
            } => {
                let (bg, color) = match kind {
                    CalloutKind::Info => ("#eef3ff", "#4263b8"),
                    CalloutKind::Warning => ("#fff5df", "#966719"),
                    CalloutKind::Error => ("#ffeeee", "#b74b4b"),
                    CalloutKind::Success => ("#eaf7f0", "#2b8059"),
                };
                let start = self.svg.len();
                let mut h = 20.;
                if let Some(t) = title {
                    h += self.text(t, x + 20., y + h, w - 40., 15., true, color) + 8.;
                }
                h += self.text(body, x + 20., y + h, w - 40., 14., false, color) + 20.;
                let contents = self.svg.split_off(start);
                self.rect(x, y, w, h, bg, false);
                self.svg.push_str(&contents);
                h
            }
            Button { label, .. } => {
                let bw = (self.width(label, 14., true) + 40.).min(w);
                let h = self.lines(label, bw - 40., 14., true).len() as f32 * 19.6 + 24.;
                self.rect(x, y, bw, h, "#5468da", false);
                self.text(label, x + 20., y + 12., bw - 40., 14., true, "#ffffff");
                h
            }
            Chart {
                title, data, unit, ..
            } => {
                let start = self.svg.len();
                let mut h =
                    24. + self.text(title, x + 24., y + 24., w - 48., 18., true, "#202a40") + 16.;
                let maximum = data.iter().map(|d| d.value).fold(0., f64::max);
                let label_w = ((w - 48.) * 0.23).max(1.);
                let value_w = ((w - 48.) * 0.22).max(1.);
                let bar_w = (w - 80. - label_w - value_w).max(1.);
                for datum in data {
                    let lh =
                        self.text(&datum.label, x + 24., y + h, label_w, 13., false, "#202a40");
                    let value = unit
                        .as_ref()
                        .map(|u| format!("{} {u}", datum.value))
                        .unwrap_or(datum.value.to_string());
                    let vh = self.text(
                        &value,
                        x + w - 24. - value_w,
                        y + h,
                        value_w,
                        13.,
                        true,
                        "#202a40",
                    );
                    self.rect(x + 40. + label_w, y + h + 3., bar_w, 16., "#f5f7fb", false);
                    if maximum > 0. && datum.value > 0. {
                        self.rect(
                            x + 40. + label_w,
                            y + h + 3.,
                            bar_w * (datum.value / maximum) as f32,
                            16.,
                            "#5468da",
                            false,
                        );
                    }
                    h += lh.max(vh).max(22.) + 16.;
                }
                h += 24.;
                let contents = self.svg.split_off(start);
                self.rect(x, y, w, h, "#ffffff", true);
                self.svg.push_str(&contents);
                h
            }
            Table {
                title,
                columns,
                rows,
                ..
            } => {
                let start = self.svg.len();
                let mut h =
                    24. + self.text(title, x + 24., y + 24., w - 48., 18., true, "#202a40") + 16.;
                let cw = (w - 48.) / columns.len().max(1) as f32;
                for (i, row) in std::iter::once(columns).chain(rows).enumerate() {
                    let rh = row
                        .iter()
                        .map(|c| self.lines(c, (cw - 24.).max(1.), 13., i == 0).len() as f32 * 18.2)
                        .fold(0., f32::max)
                        + 24.;
                    self.rect(
                        x + 24.,
                        y + h,
                        w - 48.,
                        rh,
                        if i == 0 {
                            "#f5f7fb"
                        } else if i % 2 == 0 {
                            "#fafbfe"
                        } else {
                            "#ffffff"
                        },
                        true,
                    );
                    for (col, c) in row.iter().enumerate() {
                        self.text(
                            c,
                            x + 36. + col as f32 * cw,
                            y + h + 12.,
                            cw - 24.,
                            13.,
                            i == 0,
                            "#202a40",
                        );
                    }
                    h += rh;
                }
                h += 24.;
                let contents = self.svg.split_off(start);
                self.rect(x, y, w, h, "#ffffff", true);
                self.svg.push_str(&contents);
                h
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delimited_export_preserves_quotes_newlines_and_unicode() {
        let node = ArtifactNode::Table {
            id: "t".into(),
            title: "T".into(),
            columns: vec!["Name".into(), "Value".into()],
            rows: vec![vec!["café, \"東京\"".into(), "a\tb\nc".into()]],
        };
        assert_eq!(
            table_data(&node, true).unwrap(),
            "Name,Value\r\n\"café, \"\"東京\"\"\",\"a\tb\nc\"\r\n"
        );
        assert!(table_data(&node, false).unwrap().contains("\"a\tb\nc\""));
    }
    #[test]
    fn full_document_export_includes_offscreen_rows_and_zero_charts() {
        let node = ArtifactNode::Table {
            id: "t".into(),
            title: "Unicode café 東京".into(),
            columns: vec!["Name".into()],
            rows: (0..50)
                .map(|i| vec![format!("Row {i}: a sufficiently long sentence to wrap").repeat(4)])
                .collect(),
        };
        let bytes = png(&node).unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert!(image.height() > 4000);
        assert_eq!(image.width(), 1920);
        let chart = ArtifactNode::Chart {
            id: "c".into(),
            title: "Zero".into(),
            kind: crate::artifact::ChartKind::Bar,
            unit: None,
            data: vec![crate::artifact::ChartDatum {
                label: "0".into(),
                value: 0.,
            }],
        };
        assert!(png(&chart).is_ok());
    }
}
