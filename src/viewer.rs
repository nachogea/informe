//! Document navigation and search; independent of optional review tools.
use crate::{
    artifact::{Artifact, ArtifactNode},
    preview::Preview,
    render::*,
};
use gpui::{prelude::*, *};
use gpui_base::input::{Input, InputEvent, InputState};
use std::collections::HashMap;
#[derive(Default)]
pub struct ViewerUi {
    pub scroll: ScrollHandle,
    pub anchors: HashMap<String, ScrollAnchor>,
    pub outline: Vec<(String, String, u8)>,
    pub show_outline: bool,
    pub menu: bool,
    pub find: bool,
    pub sources: bool,
    pub query: Option<Entity<InputState>>,
    pub matches: Vec<(String, String)>,
    pub current: usize,
    pub width: f32,
    pub markdown: HashMap<String, (String, Entity<gpui_base::text::TextViewState>)>,
    pub table_sort: HashMap<String, (usize, bool)>,
    search_index: Vec<(String, String)>,
    markdown_revision: usize,
    markdown_ready: bool,
    markdown_initial: HashMap<String, gpui_base::text::RenderedText>,
}
impl ViewerUi {
    pub fn rebuild(&mut self, a: &Artifact) {
        self.outline.clear();
        self.search_index.clear();
        self.matches.clear();
        self.current = 0;
        let old = std::mem::take(&mut self.anchors);
        fn walk(ui: &mut ViewerUi, n: &ArtifactNode, old: &HashMap<String, ScrollAnchor>) {
            ui.anchors.insert(
                n.id().into(),
                old.get(n.id())
                    .cloned()
                    .unwrap_or_else(|| ScrollAnchor::for_handle(ui.scroll.clone())),
            );
            match n {
                ArtifactNode::Heading { level, text, .. } if *level <= 3 => {
                    ui.outline.push((n.id().into(), text.clone(), *level))
                }
                ArtifactNode::Chart { title, .. } | ArtifactNode::Table { title, .. } => {
                    ui.outline.push((n.id().into(), title.clone(), 2))
                }
                _ => {}
            }
            if n.children().is_empty() {
                ui.search_index
                    .push((n.id().into(), n.text_content().to_lowercase()));
            }
            for c in n.children() {
                walk(ui, c, old);
            }
        }
        walk(self, &a.root, &old);
    }
}
impl ViewerUi {
    pub(crate) fn search(&self, query: &str) -> Vec<(String, String)> {
        let q = query.to_lowercase();
        if q.trim().is_empty() {
            return vec![];
        }
        self.search_index
            .iter()
            .filter(|(_, text)| text.contains(&q))
            .map(|(id, text)| (id.clone(), text.chars().take(150).collect()))
            .collect()
    }
}
impl Preview {
    pub fn prepare_markdown(&mut self, cx: &mut Context<Self>) {
        if self.viewer.markdown_revision == self.revision {
            return;
        }
        self.viewer.markdown_revision = self.revision;
        self.viewer.markdown_ready = false;
        fn blocks(n: &ArtifactNode, out: &mut Vec<(String, String)>) {
            if let ArtifactNode::Markdown { id, source } = n {
                out.push((id.clone(), source.clone()));
            }
            for c in n.children() {
                blocks(c, out);
            }
        }
        let mut sources = vec![];
        if let Some(a) = &self.artifact {
            blocks(&a.root, &mut sources);
        }
        self.viewer
            .markdown
            .retain(|id, _| sources.iter().any(|(i, _)| i == id));
        self.viewer
            .markdown_initial
            .retain(|id, _| self.viewer.markdown.contains_key(id));
        for (id, source) in sources {
            if self
                .viewer
                .markdown
                .get(&id)
                .is_none_or(|(old, _)| old != &source)
            {
                // Capture a baseline before supplying the source. Small blocks
                // parse synchronously, including rules and empty code blocks
                // whose rendered text stays empty. Snapshot identity tracks
                // the committed parse revision rather than visible text.
                let state = cx.new(|cx| gpui_base::text::TextViewState::markdown("", cx));
                let initial = state.read(cx).rendered_text();
                cx.observe(&state, |_, _, cx| cx.notify()).detach();
                state.update(cx, |state, cx| state.set_text(&source, cx));
                self.viewer.markdown_initial.insert(id.clone(), initial);
                self.viewer.markdown.insert(id, (source, state));
            }
        }
    }
    pub fn markdown_ready(&mut self, cx: &App) -> bool {
        if self.viewer.markdown_ready {
            return true;
        }
        let ready = self.viewer.markdown.iter().all(|(id, (source, state))| {
            let rendered = state.read(cx).rendered_text();
            source.is_empty()
                || self
                    .viewer
                    .markdown_initial
                    .get(id)
                    .is_some_and(|initial| initial != &rendered)
        });
        self.viewer.markdown_ready = ready;
        ready
    }
    pub fn start_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.viewer.query.is_none() {
            let query = cx.new(|cx| InputState::new(window, cx).placeholder("Find in document…"));
            cx.subscribe_in(&query, window, |this, state, event, window, cx| {
                match event {
                    InputEvent::Change => {
                        this.viewer.matches = this.viewer.search(&state.read(cx).value());
                        this.viewer.current = 0;
                        this.jump_match(window, cx);
                    }
                    InputEvent::PressEnter { shift, .. } => this.next_match(*shift, window, cx),
                    _ => {}
                }
                cx.notify();
            })
            .detach();
            self.viewer.query = Some(query);
        }
        self.viewer.find = true;
        self.viewer.menu = false;
        self.viewer
            .query
            .as_ref()
            .unwrap()
            .update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }
    pub fn jump_match(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id, _)) = self.viewer.matches.get(self.viewer.current)
            && let Some(anchor) = self.viewer.anchors.get(id)
        {
            anchor.scroll_to(window, cx);
        }
    }
    fn next_match(&mut self, previous: bool, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.viewer.matches.len();
        if len > 0 {
            self.viewer.current = (self.viewer.current + if previous { len - 1 } else { 1 }) % len;
            self.jump_match(window, cx);
        }
    }
    pub fn navigation(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("document-outline")
            .w(px(196.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .px_4()
            .py_6()
            .bg(rgb(PAPER))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .mb_4()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(MUTED))
                    .child("ON THIS PAGE"),
            )
            .children(self.viewer.outline.iter().map(|(id, title, level)| {
                let id = id.clone();
                div()
                    .id(SharedString::from(format!("nav:{id}")))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("nav:{id}")
                    })
                    .py_2()
                    .pl(px(if *level >= 3 { 12. } else { 0. }))
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .cursor_pointer()
                    .hover(|d| d.text_color(rgb(INK)))
                    .child(title.clone())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(anchor) = this.viewer.anchors.get(&id) {
                            anchor.scroll_to(window, cx);
                        }
                        if this.viewer.width < 760. {
                            this.viewer.show_outline = false;
                        }
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }
    pub fn find_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let count = self.viewer.matches.len();
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .py_2()
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .id("find-input")
                    .debug_selector(|| "find-input".into())
                    .flex_1()
                    .max_w(px(280.))
                    .min_w(px(80.))
                    .h(px(28.))
                    .child(Input::new(self.viewer.query.as_ref().unwrap())),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(if count == 0 {
                        "No matches".into()
                    } else {
                        format!("{} / {count}", self.viewer.current + 1)
                    }),
            )
            .child(
                button("find-previous", "↑")
                    .on_click(cx.listener(|this, _, w, cx| this.next_match(true, w, cx))),
            )
            .child(
                button("find-next", "↓")
                    .on_click(cx.listener(|this, _, w, cx| this.next_match(false, w, cx))),
            )
            .child(
                button("find-close", "Done").on_click(cx.listener(|this, _, _, cx| {
                    this.viewer.find = false;
                    this.focus_requested = true;
                    cx.notify();
                })),
            )
            .into_any_element()
    }
}
pub fn button(id: &'static str, label: &str) -> gpui_base::Button {
    gpui_base::Button::new(id)
        .accessibility_label(label.to_owned())
        .px_3()
        .py_1()
        .rounded_md()
        .text_size(px(12.))
        .text_color(rgb(MUTED))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(BACKGROUND)).text_color(rgb(INK)))
        .focus_visible(|d| d.bg(rgb(BACKGROUND)).border_1().border_color(rgb(ACCENT)))
        .child(label.to_owned())
}
pub fn slug(title: &str) -> String {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::ViewerUi;
    use crate::artifact::Artifact;
    #[test]
    fn search_matches_leaf_content_and_outline_has_data_sections() {
        let a = Artifact::parse(include_bytes!("../artifacts/cost-breakdown.json")).unwrap();
        let mut ui = ViewerUi::default();
        ui.rebuild(&a);
        assert!(ui.outline.iter().any(|(id, _, _)| id == "service-chart"));
        assert!(ui.search("ec2").iter().any(|(id, _)| id == "service-table"));
        assert!(
            ui.search("forecast breakdown")
                .iter()
                .any(|(id, _)| id == "service-table")
        );
        assert!(ui.search("").is_empty());
        assert!(ui.search("unmatchable123").is_empty());
    }
}
