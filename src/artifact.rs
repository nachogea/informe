//! The artifact contract has no dependency on GPUI.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub version: u32,
    pub root: ArtifactNode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ArtifactMetadata>,
}

/// Optional, author-supplied identity and provenance. Never inferred as verified.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMetadata {
    pub id: String,
    pub revision: u64,
    #[serde(default)]
    pub sources: Vec<ArtifactSource>,
    pub generated_at: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactSource {
    pub label: String,
    pub uri: Option<String>,
    pub as_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactNode {
    Column {
        id: String,
        gap: Option<f32>,
        children: Vec<ArtifactNode>,
    },
    Row {
        id: String,
        gap: Option<f32>,
        children: Vec<ArtifactNode>,
    },
    Heading {
        id: String,
        level: u8,
        text: String,
    },
    Markdown {
        id: String,
        source: String,
    },
    Text {
        id: String,
        text: String,
    },
    Card {
        id: String,
        children: Vec<ArtifactNode>,
    },
    Metric {
        id: String,
        label: String,
        value: String,
        secondary: Option<String>,
    },
    Callout {
        id: String,
        kind: CalloutKind,
        title: Option<String>,
        body: String,
    },
    Divider {
        id: String,
    },
    Button {
        id: String,
        label: String,
        action: ArtifactAction,
    },
    Chart {
        id: String,
        title: String,
        kind: ChartKind,
        unit: Option<String>,
        data: Vec<ChartDatum>,
    },
    Table {
        id: String,
        title: String,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartDatum {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalloutKind {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactAction {
    Noop,
}

impl Artifact {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let artifact: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if artifact.version != 1 {
            return Err(format!(
                "Unsupported artifact version {}; expected 1",
                artifact.version
            ));
        }
        if let Some(metadata) = &artifact.metadata {
            if metadata.id.trim().is_empty() || metadata.revision == 0 {
                return Err("metadata requires a nonempty id and revision >= 1".into());
            }
            if metadata.sources.iter().any(|s| s.label.trim().is_empty()) {
                return Err("Source labels must not be empty".into());
            }
        }
        artifact.root.validate("root", &mut HashSet::new())?;
        Ok(artifact)
    }

    /// Markdown files are imported into native semantic blocks, never HTML.
    pub fn parse_document(bytes: &[u8], path: &std::path::Path) -> Result<Self, String> {
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
        {
            return Self::parse(bytes);
        }
        let source = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let tree = markdown::to_mdast(source, &markdown::ParseOptions::gfm())
            .map_err(|e| e.to_string())?;
        let definitions = tree
            .children()
            .into_iter()
            .flatten()
            .filter(|b| matches!(b, markdown::mdast::Node::Definition(_)))
            .filter_map(|b| {
                let p = b.position()?;
                source.get(p.start.offset..p.end.offset)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut counts = std::collections::HashMap::<String, usize>::new();
        let children = tree
            .children()
            .into_iter()
            .flatten()
            .filter_map(|block| {
                if matches!(block, markdown::mdast::Node::Definition(_)) {
                    return None;
                }
                let position = block.position()?;
                let text = source
                    .get(position.start.offset..position.end.offset)?
                    .to_owned();
                use sha2::Digest;
                let hash = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
                let count = counts.entry(hash.clone()).or_default();
                *count += 1;
                let id = format!("md-{}-{}", &hash[..16], count);
                Some(match block {
                    markdown::mdast::Node::Heading(h) => ArtifactNode::Heading {
                        id,
                        level: h.depth,
                        text: block.to_string(),
                    },
                    markdown::mdast::Node::Html(_) => ArtifactNode::Text { id, text },
                    _ => ArtifactNode::Markdown {
                        id,
                        source: if definitions.is_empty() {
                            text
                        } else {
                            format!("{text}\n\n{definitions}")
                        },
                    },
                })
            })
            .collect();
        Ok(Self {
            version: 1,
            metadata: None,
            root: ArtifactNode::Column {
                id: "document".into(),
                gap: Some(20.),
                children,
            },
        })
    }

    pub fn find(&self, id: &str) -> Option<&ArtifactNode> {
        self.root.find(id)
    }

    pub fn reconcile_selection(&self, selected: &mut Option<String>) {
        if selected
            .as_deref()
            .is_some_and(|id| self.find(id).is_none())
        {
            *selected = None;
        }
    }
}

impl ArtifactNode {
    /// Human-readable content, independent of the renderer. Tables export TSV
    /// with a header row so they can be pasted directly into a spreadsheet.
    pub fn text_content(&self) -> String {
        match self {
            Self::Column { children, .. }
            | Self::Row { children, .. }
            | Self::Card { children, .. } => children
                .iter()
                .map(Self::text_content)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n"),
            Self::Heading { text, .. } | Self::Text { text, .. } => text.clone(),
            Self::Markdown { source, .. } => markdown_plain(source),
            Self::Metric {
                label,
                value,
                secondary,
                ..
            } => {
                let mut text = format!("{label}\n{value}");
                if let Some(secondary) = secondary {
                    text.push_str(&format!("\n{secondary}"));
                }
                text
            }
            Self::Callout { title, body, .. } => match title {
                Some(title) => format!("{title}\n{body}"),
                None => body.clone(),
            },
            Self::Button { label, .. } => label.clone(),
            Self::Divider { .. } => String::new(),
            Self::Chart {
                title, unit, data, ..
            } => {
                let mut lines = vec![title.clone()];
                lines.extend(data.iter().map(|datum| match unit {
                    Some(unit) => format!("{}\t{} {unit}", datum.label, datum.value),
                    None => format!("{}\t{}", datum.label, datum.value),
                }));
                lines.join("\n")
            }
            Self::Table {
                title,
                columns,
                rows,
                ..
            } => format!(
                "{title}\n{}",
                std::iter::once(columns)
                    .chain(rows.iter())
                    .map(|row| {
                        row.iter()
                            .map(|cell| cell.replace(['\t', '\r', '\n'], " "))
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Column { id, .. }
            | Self::Row { id, .. }
            | Self::Heading { id, .. }
            | Self::Markdown { id, .. }
            | Self::Text { id, .. }
            | Self::Card { id, .. }
            | Self::Metric { id, .. }
            | Self::Callout { id, .. }
            | Self::Divider { id }
            | Self::Button { id, .. }
            | Self::Chart { id, .. }
            | Self::Table { id, .. } => id,
        }
    }

    pub fn node_type(&self) -> &'static str {
        match self {
            Self::Column { .. } => "column",
            Self::Row { .. } => "row",
            Self::Heading { .. } => "heading",
            Self::Text { .. } => "text",
            Self::Markdown { .. } => "markdown",
            Self::Card { .. } => "card",
            Self::Metric { .. } => "metric",
            Self::Callout { .. } => "callout",
            Self::Divider { .. } => "divider",
            Self::Button { .. } => "button",
            Self::Chart { .. } => "chart",
            Self::Table { .. } => "table",
        }
    }

    pub fn children(&self) -> &[Self] {
        match self {
            Self::Column { children, .. }
            | Self::Row { children, .. }
            | Self::Card { children, .. } => children,
            _ => &[],
        }
    }

    fn find(&self, id: &str) -> Option<&Self> {
        if self.id() == id {
            Some(self)
        } else {
            self.children().iter().find_map(|n| n.find(id))
        }
    }

    fn validate(&self, path: &str, ids: &mut HashSet<String>) -> Result<(), String> {
        let fail = |message: &str| format!("{path} (id {:?}): {message}", self.id());
        if self.id().trim().is_empty() {
            return Err(fail("ID must not be empty"));
        }
        if !ids.insert(self.id().to_owned()) {
            return Err(fail("duplicate node ID"));
        }
        match self {
            Self::Column { gap: Some(gap), .. } | Self::Row { gap: Some(gap), .. }
                if !gap.is_finite() || *gap < 0. =>
            {
                return Err(fail("gap must be finite and nonnegative"));
            }
            Self::Heading { level, .. } if !(1..=6).contains(level) => {
                return Err(fail("heading level must be 1–6"));
            }
            Self::Chart { data, .. } => {
                if data.is_empty() {
                    return Err(fail("chart data must not be empty"));
                }
                for (i, datum) in data.iter().enumerate() {
                    if !datum.value.is_finite() || datum.value < 0. {
                        return Err(fail(&format!(
                            "data[{i}].value must be finite and nonnegative"
                        )));
                    }
                }
            }
            Self::Table { columns, rows, .. } => {
                if columns.is_empty() {
                    return Err(fail("table must have at least one column"));
                }
                for (i, row) in rows.iter().enumerate() {
                    if row.len() != columns.len() {
                        return Err(fail(&format!(
                            "rows[{i}] has {} cells; expected {}",
                            row.len(),
                            columns.len()
                        )));
                    }
                }
            }
            _ => {}
        }
        for (i, child) in self.children().iter().enumerate() {
            child.validate(&format!("{path}.children[{i}]"), ids)?;
        }
        Ok(())
    }
}

pub fn markdown_plain(source: &str) -> String {
    fn plain(n: &markdown::mdast::Node) -> String {
        use markdown::mdast::Node;
        match n {
            Node::Root(x) => x
                .children
                .iter()
                .map(plain)
                .collect::<Vec<_>>()
                .join("\n\n"),
            Node::List(x) => x
                .children
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    format!(
                        "{} {}",
                        if x.ordered {
                            format!("{}.", x.start.unwrap_or(1) + i as u32)
                        } else {
                            "•".into()
                        },
                        plain(n)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Node::ListItem(x) => {
                let content = x.children.iter().map(plain).collect::<Vec<_>>().join("\n");
                match x.checked {
                    Some(true) => format!("[x] {content}"),
                    Some(false) => format!("[ ] {content}"),
                    None => content,
                }
            }
            Node::Blockquote(x) => x.children.iter().map(plain).collect::<Vec<_>>().join("\n"),
            Node::Table(x) => x.children.iter().map(plain).collect::<Vec<_>>().join("\n"),
            Node::TableRow(x) => x.children.iter().map(plain).collect::<Vec<_>>().join("\t"),
            Node::Break(_) => "\n".into(),
            _ => n
                .children()
                .map(|children| children.iter().map(plain).collect::<String>())
                .unwrap_or_else(|| n.to_string()),
        }
    }
    markdown::to_mdast(source, &markdown::ParseOptions::gfm())
        .map(|n| plain(&n))
        .unwrap_or_else(|_| source.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_import_preserves_unicode_links_and_unchanged_block_identity() {
        let path = std::path::Path::new("report.MD");
        let source = "# Résumé\n\nKeep **東京** and [evidence][ref].\n\n[ref]: https://example.org\n\n<div>literal HTML</div>\n";
        let first = Artifact::parse_document(source.as_bytes(), path).unwrap();
        let second =
            Artifact::parse_document(format!("Intro added.\n\n{source}").as_bytes(), path).unwrap();
        for node in first.root.children() {
            assert_eq!(second.find(node.id()), Some(node));
        }
        assert_eq!(first.root.children()[0].text_content(), "Résumé");
        assert!(first.root.text_content().contains("東京"));
        assert!(
            first
                .root
                .children()
                .iter()
                .any(|n| matches!(n, ArtifactNode::Text {text,..} if text.contains("<div>")))
        );
        assert!(first.root.children().iter().any(|n| matches!(n, ArtifactNode::Markdown {source,..} if source.contains("[ref]: https://example.org"))));
        assert!(Artifact::parse_document(&[255], path).is_err());
        assert!(
            Artifact::parse_document(source.as_bytes(), std::path::Path::new("report.json"))
                .is_err()
        );
    }
    #[test]
    fn markdown_duplicate_blocks_have_unique_ids_and_json_roundtrips() {
        let a =
            Artifact::parse_document(b"Same.\n\nSame.", std::path::Path::new("plan.md")).unwrap();
        assert_ne!(a.root.children()[0].id(), a.root.children()[1].id());
        assert_eq!(
            Artifact::parse(&serde_json::to_vec(&a).unwrap()).unwrap(),
            a
        );
        assert_eq!(markdown_plain("- **One**\n- Two"), "• One\n• Two");
        assert_eq!(
            markdown_plain("- [x] Completed\n- [ ] Pending"),
            "• [x] Completed\n• [ ] Pending"
        );
    }

    #[test]
    fn exports_table_as_rectangular_tsv_and_container_as_readable_text() {
        let artifact = Artifact::parse(br#"{"version":1,"root":{"type":"column","id":"root","children":[{"type":"heading","id":"title","level":1,"text":"Summary"},{"type":"divider","id":"divider"},{"type":"table","id":"table","title":"Costs","columns":["Service","Spend"],"rows":[["EC2\tcompute","$188,000\nmonthly"]]}]}}"#).unwrap();
        assert_eq!(
            artifact.find("table").unwrap().text_content(),
            "Costs\nService\tSpend\nEC2 compute\t$188,000 monthly"
        );
        assert_eq!(
            artifact.root.text_content(),
            "Summary\n\nCosts\nService\tSpend\nEC2 compute\t$188,000 monthly"
        );
    }

    #[test]
    fn examples_parse_and_have_stable_lookup() {
        let dashboard = Artifact::parse(include_bytes!("../examples/dashboard.json")).unwrap();
        assert_eq!(
            dashboard.find("projected-spend").unwrap().node_type(),
            "metric"
        );
        Artifact::parse(include_bytes!("../examples/primitives.json")).unwrap();
        let forecast = Artifact::parse(include_bytes!("../artifacts/cost-breakdown.json")).unwrap();
        assert_eq!(forecast.find("service-chart").unwrap().node_type(), "chart");
        assert_eq!(forecast.find("service-table").unwrap().node_type(), "table");
    }

    #[test]
    fn rejects_ambiguous_ids_with_location() {
        let error = Artifact::parse(br#"{"version":1,"root":{"type":"column","id":"root","children":[{"type":"text","id":"root","text":"duplicate"}]}}"#).unwrap_err();
        assert!(error.contains("duplicate node ID"));
        assert!(error.contains("root.children[0]"));
        assert!(Artifact::parse(br#"{"version":1,"root":{"type":"divider","id":" "}}"#).is_err());
    }

    #[test]
    fn rejects_invalid_contract_values() {
        for json in [
            r#"{"version":2,"root":{"type":"divider","id":"x"}}"#,
            r#"{"version":1,"root":{"type":"heading","id":"x","level":0,"text":"x"}}"#,
            r#"{"version":1,"root":{"type":"row","id":"x","gap":-1,"children":[]}}"#,
            r#"{"version":1,"root":{"type":"text","id":"x","text":"x","color":"red"}}"#,
            r#"{"version":1,"root":{"type":"button","id":"x","label":"x","action":{"type":"run_command"}}}"#,
            r#"{"version":1,"root":{"type":"chart","id":"x"}}"#,
        ] {
            assert!(Artifact::parse(json.as_bytes()).is_err(), "accepted {json}");
        }
    }

    #[test]
    fn metadata_is_optional_but_identity_revision_and_sources_are_validated() {
        for metadata in [
            r#"{"id":"","revision":1}"#,
            r#"{"id":"report","revision":0}"#,
            r#"{"id":"report","revision":1,"sources":[{"label":" "}]}"#,
            r#"{"id":"report","revision":1,"verified":true}"#,
        ] {
            assert!(Artifact::parse(format!(r#"{{"version":1,"metadata":{metadata},"root":{{"type":"divider","id":"root"}}}}"#).as_bytes()).is_err());
        }
        let a = Artifact::parse(include_bytes!("../examples/review/cost-v1.json")).unwrap();
        assert_eq!(a.metadata.as_ref().unwrap().id, "cost-review-demo");
        assert_eq!(
            Artifact::parse(&serde_json::to_vec(&a).unwrap()).unwrap(),
            a
        );
    }

    #[test]
    fn reload_retains_identity_but_refreshes_metadata() {
        let artifact =
            Artifact::parse(br#"{"version":1,"root":{"type":"text","id":"same","text":"new"}}"#)
                .unwrap();
        let mut selected = Some("same".to_owned());
        artifact.reconcile_selection(&mut selected);
        assert_eq!(
            artifact
                .find(selected.as_deref().unwrap())
                .unwrap()
                .node_type(),
            "text"
        );
        selected = Some("deleted".to_owned());
        artifact.reconcile_selection(&mut selected);
        assert_eq!(selected, None);
    }

    #[test]
    fn validates_chart_values_and_table_shape() {
        for node in [
            r#"{"type":"chart","id":"c","title":"Costs","kind":"bar","data":[]}"#,
            r#"{"type":"chart","id":"c","title":"Costs","kind":"bar","data":[{"label":"A","value":-1}]}"#,
            r#"{"type":"chart","id":"c","title":"Costs","kind":"bar","data":[{"label":"A","value":1e309}]}"#,
            r#"{"type":"chart","id":"c","title":"Costs","kind":"pie","data":[{"label":"A","value":1}]}"#,
            r#"{"type":"table","id":"t","title":"Costs","columns":[],"rows":[]}"#,
            r#"{"type":"table","id":"t","title":"Costs","columns":["A","B"],"rows":[["x"]]}"#,
        ] {
            assert!(
                Artifact::parse(format!("{{\"version\":1,\"root\":{node}}}").as_bytes()).is_err()
            );
        }
        for node in [
            r#"{"type":"chart","id":"c","title":"Costs","kind":"bar","data":[{"label":"A","value":0}]}"#,
            r#"{"type":"table","id":"t","title":"Costs","columns":["A"],"rows":[]}"#,
        ] {
            assert!(
                Artifact::parse(format!("{{\"version\":1,\"root\":{node}}}").as_bytes()).is_ok()
            );
        }
    }
}
