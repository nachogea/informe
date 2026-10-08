//! Renderer-independent, revision-bound comments and semantic differences.
use crate::artifact::{Artifact, ArtifactNode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn fingerprint<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serializable artifact"))
    )
}
pub fn identity(artifact: &Artifact, path: &Path) -> String {
    artifact
        .metadata
        .as_ref()
        .map(|m| m.id.clone())
        .unwrap_or_else(|| {
            format!(
                "path:{}",
                path.canonicalize()
                    .unwrap_or_else(|_| path.to_owned())
                    .display()
            )
        })
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub content_hash: String,
    pub label: Option<u64>,
}
impl Revision {
    pub fn of(a: &Artifact) -> Self {
        Self {
            content_hash: fingerprint(a),
            label: a.metadata.as_ref().map(|m| m.revision),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Node {
        node_id: String,
        node_type: String,
        original: ArtifactNode,
    },
    Text {
        node_id: String,
        node_type: String,
        quote: String,
        start: usize,
        end: usize,
        prefix: String,
        suffix: String,
        original: ArtifactNode,
    },
}
impl Target {
    pub fn node_id(&self) -> &str {
        match self {
            Self::Node { node_id, .. } | Self::Text { node_id, .. } => node_id,
        }
    }
    pub fn original(&self) -> &ArtifactNode {
        match self {
            Self::Node { original, .. } | Self::Text { original, .. } => original,
        }
    }
    pub fn node(a: &Artifact, id: &str) -> Result<Self, String> {
        let node = a.find(id).ok_or("Target node does not exist")?;
        Ok(Self::Node {
            node_id: id.into(),
            node_type: node.node_type().into(),
            original: node.clone(),
        })
    }
    pub fn text(
        a: &Artifact,
        id: Option<&str>,
        quote: &str,
        offset: Option<usize>,
    ) -> Result<Self, String> {
        if quote.trim().is_empty() {
            return Err("Select some text first".into());
        }
        fn candidates<'a>(node: &'a ArtifactNode, quote: &str) -> Vec<&'a ArtifactNode> {
            let children: Vec<_> = node
                .children()
                .iter()
                .flat_map(|c| candidates(c, quote))
                .collect();
            if !children.is_empty() {
                children
            } else if node.text_content().contains(quote) {
                vec![node]
            } else {
                vec![]
            }
        }
        let node = if let Some(id) = id {
            a.find(id).ok_or("Target node does not exist")?
        } else {
            let candidates = candidates(&a.root, quote);
            if candidates.len() != 1 {
                return Err("Selected quote is ambiguous or spans unsupported regions; select a component instead".into());
            }
            candidates[0]
        };
        let content = node.text_content();
        let matches: Vec<_> = content.match_indices(quote).map(|(i, _)| i).collect();
        let start = if let Some(offset) = offset {
            if !matches.contains(&offset) {
                return Err("Quote does not match the requested byte offset".into());
            }
            offset
        } else {
            if matches.len() != 1 {
                return Err(
                    "Quote is ambiguous; supply a byte offset or select a narrower component"
                        .into(),
                );
            }
            matches[0]
        };
        let end = start + quote.len();
        Ok(Self::Text {
            node_id: node.id().into(),
            node_type: node.node_type().into(),
            quote: quote.into(),
            start,
            end,
            prefix: content[..start]
                .chars()
                .rev()
                .take(32)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
            suffix: content[end..].chars().take(32).collect(),
            original: node.clone(),
        })
    }
    pub fn status(&self, a: &Artifact) -> TargetStatus {
        let Some(node) = a.find(self.node_id()) else {
            return TargetStatus::Removed;
        };
        if node.node_type() != self.original().node_type() {
            return TargetStatus::TypeChanged;
        }
        let changed = node != self.original();
        if let Self::Text {
            quote,
            prefix,
            suffix,
            ..
        } = self
        {
            let text = node.text_content();
            let locations: Vec<_> = text.match_indices(quote).map(|(i, _)| i).collect();
            if locations.is_empty() {
                return TargetStatus::QuoteChanged;
            }
            if locations.len() > 1 {
                let contextual = locations
                    .iter()
                    .filter(|&&i| {
                        text[..i].ends_with(prefix) && text[i + quote.len()..].starts_with(suffix)
                    })
                    .count();
                if contextual != 1 {
                    return TargetStatus::Ambiguous;
                }
            }
        }
        if changed {
            TargetStatus::Changed
        } else {
            TargetStatus::Attached
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetStatus {
    Attached,
    Changed,
    Removed,
    TypeChanged,
    QuoteChanged,
    Ambiguous,
}
impl TargetStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Attached => "Target unchanged",
            Self::Changed => "Target changed — review context",
            Self::Removed => "Target removed",
            Self::TypeChanged => "Component type changed",
            Self::QuoteChanged => "Quoted text changed or removed",
            Self::Ambiguous => "Quote now ambiguous",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comment {
    pub id: String,
    pub revision: Revision,
    pub target: Target,
    pub body: String,
    pub resolved: bool,
    pub created_at_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackComment {
    pub comment: Comment,
    pub target_status: TargetStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feedback {
    pub version: u32,
    pub id: String,
    pub artifact_id: String,
    pub artifact_path: PathBuf,
    pub revision: Revision,
    pub metadata: Option<crate::artifact::ArtifactMetadata>,
    pub comments: Vec<FeedbackComment>,
    pub submitted_at_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub version: u32,
    pub artifact_id: String,
    pub baseline: Artifact,
    pub comments: Vec<Comment>,
    pub submission: Option<Feedback>,
}
impl Review {
    pub fn new(a: &Artifact, path: &Path) -> Self {
        Self {
            version: 1,
            artifact_id: identity(a, path),
            baseline: a.clone(),
            comments: Vec::new(),
            submission: None,
        }
    }
    pub fn add(&mut self, a: &Artifact, target: Target, body: &str) -> Result<String, String> {
        if body.trim().is_empty() {
            return Err("Comment must not be empty".into());
        }
        if body.len() > 16_000 {
            return Err("Comment exceeds 16,000 bytes".into());
        }
        let id = format!("comment-{}", stamp());
        self.comments.push(Comment {
            id: id.clone(),
            revision: Revision::of(a),
            target,
            body: body.trim().into(),
            resolved: false,
            created_at_ms: stamp() / 1_000_000,
        });
        Ok(id)
    }
    pub fn submit(&mut self, a: &Artifact, path: &Path) -> Result<Feedback, String> {
        let comments: Vec<_> = self
            .comments
            .iter()
            .filter(|c| !c.resolved)
            .map(|c| FeedbackComment {
                comment: c.clone(),
                target_status: c.target.status(a),
            })
            .collect();
        if comments.is_empty() {
            return Err("Add an unresolved comment before sending feedback".into());
        }
        let feedback = Feedback {
            version: 1,
            id: format!("feedback-{}", stamp()),
            artifact_id: self.artifact_id.clone(),
            artifact_path: path.canonicalize().unwrap_or_else(|_| path.to_owned()),
            revision: Revision::of(a),
            metadata: a.metadata.clone(),
            comments,
            submitted_at_ms: stamp() / 1_000_000,
        };
        self.submission = Some(feedback.clone());
        Ok(feedback)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Changed,
    Removed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub node_id: String,
    pub kind: ChangeKind,
    pub node_type: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}
fn semantic(node: &ArtifactNode) -> serde_json::Value {
    let mut value = serde_json::to_value(node).expect("node JSON");
    if let Some(children) = value.get_mut("children") {
        *children = serde_json::json!(
            node.children()
                .iter()
                .map(ArtifactNode::id)
                .collect::<Vec<_>>()
        );
    }
    value
}
fn index(node: &ArtifactNode, out: &mut BTreeMap<String, (String, serde_json::Value)>) {
    out.insert(node.id().into(), (node.node_type().into(), semantic(node)));
    for c in node.children() {
        index(c, out);
    }
}
pub fn changes(before: &Artifact, after: &Artifact) -> Vec<Change> {
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    index(&before.root, &mut old);
    index(&after.root, &mut new);
    let mut changes = Vec::new();
    for (id, (kind, value)) in &new {
        match old.get(id) {
            None => changes.push(Change {
                node_id: id.clone(),
                kind: ChangeKind::Added,
                node_type: kind.clone(),
                before: None,
                after: Some(value.clone()),
            }),
            Some((_, previous)) if previous != value => changes.push(Change {
                node_id: id.clone(),
                kind: ChangeKind::Changed,
                node_type: kind.clone(),
                before: Some(previous.clone()),
                after: Some(value.clone()),
            }),
            _ => {}
        }
    }
    for (id, (kind, value)) in old {
        if !new.contains_key(&id) {
            changes.push(Change {
                node_id: id,
                kind: ChangeKind::Removed,
                node_type: kind,
                before: Some(value),
                after: None,
            });
        }
    }
    changes
}

pub fn sidecar(path: &Path) -> PathBuf {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".review.json");
    path.with_file_name(name)
}
pub fn read_artifact(path: &Path) -> Result<Artifact, String> {
    Artifact::parse_document(&fs::read(path).map_err(|e| e.to_string())?, path)
}
pub fn load(path: &Path, a: &Artifact) -> Result<Review, String> {
    let file = sidecar(path);
    match fs::read(file) {
        Ok(bytes) => {
            let review: Review = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Invalid review sidecar: {e}"))?;
            if review.version != 1 {
                return Err("Unsupported review sidecar version".into());
            }
            if review.artifact_id != identity(a, path) {
                return Err("Artifact identity changed; existing comments belong to another document. Restore its identity or use a new artifact path.".into());
            }
            Ok(review)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Review::new(a, path)),
        Err(e) => Err(e.to_string()),
    }
}
pub fn edit<T>(
    path: &Path,
    expected: Option<&str>,
    operation: impl FnOnce(&mut Review, &Artifact) -> Result<T, String>,
) -> Result<(Review, T), String> {
    let file = sidecar(path);
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(file.with_extension("lock"))
        .map_err(|e| e.to_string())?;
    lock.lock().map_err(|e| e.to_string())?;
    let a = read_artifact(path)?;
    if expected.is_some_and(|hash| hash != fingerprint(&a)) {
        return Err(
            "Artifact changed while composing feedback; review the new revision first".into(),
        );
    }
    let mut review = load(path, &a)?;
    let result = operation(&mut review, &a)?;
    atomic_json(&file, &review)?;
    Ok((review, result))
}
pub fn apply_revision(path: &Path, candidate: &Path, expected: &str) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
    {
        return Err("Apply Markdown revisions by editing the Markdown source; apply-revision accepts JSON artifacts".into());
    }
    let candidate = read_artifact(candidate)?;
    let file = sidecar(&path);
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(file.with_extension("lock"))
        .map_err(|e| e.to_string())?;
    lock.lock().map_err(|e| e.to_string())?;
    let current = read_artifact(&path)?;
    if fingerprint(&current) != expected {
        return Err("Artifact changed since feedback was submitted; review the new revision before applying".into());
    }
    if identity(&current, &path) != identity(&candidate, &path) {
        return Err("Revision must preserve artifact identity".into());
    }
    if let (Some(old), Some(new)) = (&current.metadata, &candidate.metadata)
        && new.revision <= old.revision
    {
        return Err("metadata.revision must increase".into());
    }
    // Validate existing review data before changing its artifact.
    load(&path, &current)?;
    // Keep the pre-edit baseline even when the first interaction is a CLI revision.
    if !file.exists() {
        let review = Review::new(&current, &path);
        atomic_json(&file, &review)?;
    }
    atomic_json(&path, &candidate)
}
fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temporary = path.with_extension(format!("pending-{}", stamp()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn artifact(value: &str) -> Artifact {
        Artifact::parse(format!(r#"{{"version":1,"metadata":{{"id":"forecast","revision":1}},"root":{{"type":"column","id":"root","children":[{{"type":"metric","id":"spend","label":"Projected","value":"{value}"}},{{"type":"text","id":"note","text":"café cost forecast"}}]}}}}"#).as_bytes()).unwrap()
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "artifact-review-{}-{}",
                std::process::id(),
                stamp()
            ));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn path(&self) -> PathBuf {
            self.0.join("artifact.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn diff_tracks_semantics_without_highlighting_every_ancestor() {
        let before = artifact("$412,000");
        let mut after = artifact("$398,000");
        assert_eq!(
            changes(&before, &after)
                .iter()
                .map(|c| c.node_id.as_str())
                .collect::<Vec<_>>(),
            ["spend"]
        );
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children.reverse();
            children.pop();
            children.push(ArtifactNode::Divider { id: "new".into() });
        }
        let diff = changes(&before, &after);
        assert!(
            diff.iter()
                .any(|c| c.node_id == "root" && c.kind == ChangeKind::Changed)
        );
        assert!(
            diff.iter()
                .any(|c| c.node_id == "spend" && c.kind == ChangeKind::Removed)
        );
        assert!(
            diff.iter()
                .any(|c| c.node_id == "new" && c.kind == ChangeKind::Added)
        );
    }
    #[test]
    fn targets_keep_original_context_and_flag_removed_changed_and_ambiguous() {
        let before = artifact("$412,000");
        let target = Target::node(&before, "spend").unwrap();
        assert_eq!(target.status(&artifact("$398,000")), TargetStatus::Changed);
        let mut after = before.clone();
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children.remove(0);
        }
        assert_eq!(target.status(&after), TargetStatus::Removed);
        let text = Target::text(&before, Some("note"), "café", None).unwrap();
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children[0] = ArtifactNode::Text {
                id: "note".into(),
                text: "Edited café cost forecast".into(),
            };
        }
        assert_eq!(text.status(&after), TargetStatus::Changed);
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children[0] = ArtifactNode::Text {
                id: "note".into(),
                text: "café other café words".into(),
            };
        }
        assert_eq!(text.status(&after), TargetStatus::Ambiguous);
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children[0] = ArtifactNode::Text {
                id: "note".into(),
                text: "new quote".into(),
            };
        }
        assert_eq!(text.status(&after), TargetStatus::QuoteChanged);
        if let ArtifactNode::Column { children, .. } = &mut after.root {
            children[0] = ArtifactNode::Divider { id: "note".into() };
        }
        assert_eq!(text.status(&after), TargetStatus::TypeChanged);
    }
    #[test]
    fn identical_quotes_in_different_nodes_require_explicit_target() {
        let mut a = artifact("$412,000");
        if let ArtifactNode::Column { children, .. } = &mut a.root {
            children.push(ArtifactNode::Text {
                id: "duplicate".into(),
                text: "café".into(),
            });
        }
        assert!(Target::text(&a, None, "café", None).is_err());
        assert!(Target::text(&a, Some("duplicate"), "café", None).is_ok());
        if let ArtifactNode::Column { children, .. } = &mut a.root {
            children.push(ArtifactNode::Text {
                id: "repeated".into(),
                text: "same same".into(),
            });
        }
        assert!(Target::text(&a, Some("repeated"), "same", None).is_err());
        assert!(Target::text(&a, Some("repeated"), "same", Some(5)).is_ok());
    }
    #[test]
    fn submission_is_revision_bound_and_resolution_does_not_rewrite_sent_feedback() {
        let a = artifact("$412,000");
        let path = Path::new("forecast.json");
        let mut state = Review::new(&a, path);
        state
            .add(&a, Target::node(&a, "spend").unwrap(), "Compare last month")
            .unwrap();
        let sent = state.submit(&a, path).unwrap();
        state.comments[0].resolved = true;
        assert!(
            !state.submission.as_ref().unwrap().comments[0]
                .comment
                .resolved
        );
        assert_eq!(sent.revision, Revision::of(&a));
        assert!(state.submit(&a, path).is_err());
        state.comments[0].resolved = false;
        let new = state.submit(&artifact("$398,000"), path).unwrap();
        assert_eq!(new.comments[0].target_status, TargetStatus::Changed);
        assert_ne!(
            new.revision.content_hash,
            new.comments[0].comment.revision.content_hash
        );
    }
    #[test]
    fn concurrent_saves_merge_and_stale_revisions_are_rejected() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let a = artifact("$412,000");
        fs::write(&path, serde_json::to_vec(&a).unwrap()).unwrap();
        let threads: Vec<_> = (0..3)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for j in 0..3 {
                        edit(&path, None, |state, a| {
                            state.add(a, Target::node(a, "spend")?, &format!("Comment {i}.{j}"))
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        let state = load(&path, &a).unwrap();
        assert_eq!(state.comments.len(), 9);
        let candidate = fixture.0.join("next.json");
        let mut next = artifact("$398,000");
        next.metadata.as_mut().unwrap().revision = 2;
        fs::write(&candidate, serde_json::to_vec(&next).unwrap()).unwrap();
        assert!(apply_revision(&path, &candidate, "wrong").is_err());
        assert_eq!(read_artifact(&path).unwrap(), a);
        apply_revision(&path, &candidate, &fingerprint(&a)).unwrap();
        assert_eq!(read_artifact(&path).unwrap(), next);
        assert_eq!(load(&path, &next).unwrap().comments.len(), 9);
        assert_eq!(load(&path, &next).unwrap().baseline, a);
        assert!(apply_revision(&path, &candidate, &fingerprint(&next)).is_err());
    }
    #[test]
    fn corrupt_sidecars_and_identity_changes_never_overwrite_feedback() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let a = artifact("$412,000");
        fs::write(&path, serde_json::to_vec(&a).unwrap()).unwrap();
        edit(&path, None, |_, _| Ok(())).unwrap();
        let old = fs::read(sidecar(&path)).unwrap();
        let mut foreign = a.clone();
        foreign.metadata.as_mut().unwrap().id = "another".into();
        assert!(load(&path, &foreign).is_err());
        assert_eq!(fs::read(sidecar(&path)).unwrap(), old);
        fs::write(sidecar(&path), b"corrupt").unwrap();
        assert!(edit(&path, None, |_, _| Ok(())).is_err());
        assert_eq!(fs::read(sidecar(&path)).unwrap(), b"corrupt");
        let candidate = fixture.0.join("next.json");
        let mut next = a.clone();
        next.metadata.as_mut().unwrap().revision = 2;
        fs::write(&candidate, serde_json::to_vec(&next).unwrap()).unwrap();
        assert!(apply_revision(&path, &candidate, &fingerprint(&a)).is_err());
        assert_eq!(read_artifact(&path).unwrap(), a);
    }
}
