//! Human-facing review UI, separate from semantic rendering and review storage.
use crate::{
    preview::Preview,
    render::*,
    review::{self, Change, ChangeKind, Review, Target},
};
use gpui::{prelude::*, *};
use gpui_base::input::{Textarea, TextareaState};
#[derive(Default)]
pub struct ReviewUi {
    pub state: Option<Review>,
    pub error: Option<String>,
    pub changes: Vec<Change>,
    pub show: bool,
    pub draft: Option<(Target, String)>,
    pub editor: Option<Entity<TextareaState>>,
    pub busy: bool,
    pub quote: String,
    pub show_changes: bool,
}
impl Preview {
    pub fn apply_review(&mut self, result: Result<Review, String>) -> bool {
        match result {
            Ok(state) => {
                let hash = review::fingerprint(&state);
                let changed = self
                    .review
                    .state
                    .as_ref()
                    .is_none_or(|s| review::fingerprint(s) != hash)
                    || self.review.error.is_some();
                self.review.state = Some(state);
                self.review.error = None;
                self.refresh_changes();
                changed
            }
            Err(error) => {
                let changed = self.review.error.as_ref() != Some(&error);
                self.review.error = Some(error);
                self.review.changes.clear();
                changed
            }
        }
    }
    pub fn refresh_changes(&mut self) {
        self.review.changes = match (&self.review.state, &self.artifact) {
            (Some(state), Some(a)) if review::identity(a, &self.path) == state.artifact_id => {
                review::changes(&state.baseline, a)
            }
            _ => Vec::new(),
        };
    }
    pub fn begin_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.review.busy || self.review.error.is_some() {
            self.status = "Review data is unavailable; check the comments panel".into();
            self.review.show = true;
            cx.notify();
            return;
        }
        if self.review.draft.is_some() {
            self.review.show = true;
            cx.notify();
            return;
        }
        let Some(a) = &self.artifact else {
            return;
        };
        let target = if self.component_mode {
            Target::node(a, self.selected_id.as_deref().unwrap_or(a.root.id()))
        } else if !self.review.quote.is_empty() {
            Target::text(a, None, &self.review.quote, None)
        } else {
            Target::node(a, a.root.id())
        };
        match target {
            Ok(target) => {
                self.review.draft = Some((target, review::fingerprint(a)));
                let editor = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .placeholder("What should change?")
                        .rows(4)
                });
                editor.update(cx, |state, cx| state.focus(window, cx));
                self.review.editor = Some(editor);
                self.review.quote.clear();
                self.review.show = true;
                gpui_base::TextSelection::clear(window, cx);
            }
            Err(error) => self.status = error,
        }
        cx.notify();
    }
    fn review_operation(
        &mut self,
        operation: impl FnOnce(&mut Review, &crate::artifact::Artifact) -> Result<(), String>
        + Send
        + 'static,
        expected: String,
        clear_draft: bool,
        cx: &mut Context<Self>,
    ) {
        if self.review.busy {
            return;
        }
        self.review.busy = true;
        let path = self.path.clone();
        cx.notify();
        cx.spawn(async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { review::edit(&path, Some(&expected), operation) })
                .await;
            let _ = entity.update(cx, |this, cx| {
                this.review.busy = false;
                match result {
                    Ok((state, ())) => {
                        this.apply_review(Ok(state));
                        this.status = "Review saved".into();
                        if clear_draft {
                            this.review.draft = None;
                            this.review.editor = None;
                            this.focus_requested = true;
                        }
                    }
                    Err(error) => this.status = format!("Review not saved: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn save_comment(&mut self, cx: &mut Context<Self>) {
        let (Some((target, hash)), Some(editor)) = (&self.review.draft, &self.review.editor) else {
            return;
        };
        let body = editor.read(cx).value().to_string();
        let target = target.clone();
        let hash = hash.clone();
        self.review_operation(
            move |state, a| state.add(a, target, &body).map(|_| ()),
            hash,
            true,
            cx,
        );
    }
    fn change_review(&mut self, action: &str, id: Option<String>, cx: &mut Context<Self>) {
        let Some(a) = &self.artifact else {
            return;
        };
        let hash = review::fingerprint(a);
        let path = self.path.clone();
        let action = action.to_owned();
        self.review_operation(
            move |state, a| {
                match action.as_str() {
                    "submit" => {
                        state.submit(a, &path)?;
                    }
                    "accept" => state.baseline = a.clone(),
                    "resolve" => {
                        let comment = state
                            .comments
                            .iter_mut()
                            .find(|c| Some(&c.id) == id.as_ref())
                            .ok_or("Comment no longer exists")?;
                        comment.resolved = !comment.resolved;
                    }
                    _ => unreachable!(),
                }
                Ok(())
            },
            hash,
            false,
            cx,
        );
    }
    pub fn review_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        let count = self
            .review
            .state
            .as_ref()
            .map(|s| s.comments.iter().filter(|c| !c.resolved).count())
            .unwrap_or(0);
        let selected = self.component_mode && self.selected_id.is_some();
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .py_2()
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(BORDER))
            .text_size(px(12.))
            .child(
                review_button(
                    "comment",
                    if selected {
                        "Comment on component…"
                    } else {
                        "Comment…"
                    },
                )
                .on_click(cx.listener(|this, _, window, cx| this.begin_comment(window, cx))),
            )
            .child(
                review_button("comments-toggle", &format!("Comments ({count})")).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.review.show = !this.review.show;
                        cx.notify();
                    }),
                ),
            )
            .child(
                review_button(
                    "changes-toggle",
                    &format!("{} changes", self.review.changes.len()),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.review.show = true;
                    this.review.show_changes = !this.review.show_changes;
                    cx.notify();
                })),
            )
            .child(div().flex_1())
            .child(div().text_color(rgb(MUTED)).child(if self.review.busy {
                "Saving review…"
            } else {
                "Select a component or text to comment · ⌘⇧M"
            }))
            .into_any_element()
    }
    pub fn review_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div()
            .id("comments-panel")
            .w(px(340.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .p_5()
            .bg(rgb(PAPER))
            .border_l_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_4()
            .text_size(px(13.))
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Review"),
            );
        if let Some(error) = &self.review.error {
            panel = panel.child(div().text_color(rgb(0xb74b4b)).child(error.clone()));
        }
        if let Some((target, _)) = &self.review.draft {
            let context = match target {
                Target::Text { quote, .. } => format!("On selected text: “{quote}”"),
                Target::Node { original, .. } => format!(
                    "On {}: {}",
                    original.node_type(),
                    original
                        .text_content()
                        .chars()
                        .take(120)
                        .collect::<String>()
                ),
            };
            panel = panel.child(div().p_3().bg(rgb(BACKGROUND)).rounded_md().child(context));
            if let Some(editor) = &self.review.editor {
                panel = panel.child(
                    div()
                        .id("comment-editor")
                        .debug_selector(|| "comment-editor".into())
                        .h(px(120.))
                        .p_2()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .rounded_md()
                        .child(Textarea::new(editor)),
                );
            }
            panel = panel.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        review_button("save-comment", "Add comment")
                            .on_click(cx.listener(|this, _, _, cx| this.save_comment(cx))),
                    )
                    .child(
                        review_button("cancel-comment", "Cancel").on_click(cx.listener(
                            |this, _, _, cx| {
                                if !this.review.busy {
                                    this.review.draft = None;
                                    this.review.editor = None;
                                    this.focus_requested = true;
                                    cx.notify();
                                }
                            },
                        )),
                    ),
            );
        }
        if let (Some(state), Some(a)) = (&self.review.state, &self.artifact) {
            panel = panel.child(
                review_button("submit-feedback", "Send feedback to agent")
                    .on_click(cx.listener(|this, _, _, cx| this.change_review("submit", None, cx))),
            );
            if let Some(batch) = &state.submission {
                panel = panel
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .text_size(px(11.))
                            .child(format!(
                                "Last sent: {} comments · revision {}",
                                batch.comments.len(),
                                batch
                                    .revision
                                    .label
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| batch.revision.content_hash[..8].into())
                            )),
                    )
                    .child(
                        review_button("copy-feedback", "Copy submitted feedback").on_click(
                            cx.listener(|this, _, _, cx| {
                                if let Some(batch) = this
                                    .review
                                    .state
                                    .as_ref()
                                    .and_then(|s| s.submission.as_ref())
                                {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        serde_json::to_string_pretty(batch).expect("feedback JSON"),
                                    ));
                                    this.status = "Copied submitted feedback JSON".into();
                                    cx.notify();
                                }
                            }),
                        ),
                    );
            }
            if state.comments.is_empty() {
                panel=panel.child(div().text_color(rgb(MUTED)).child("Comment on a component or selected text. Send feedback when you’re ready for your agent to revise it."));
            }
            for comment in &state.comments {
                let id = comment.id.clone();
                let target = comment.target.clone();
                let mut card = div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if comment.resolved {
                                "Resolved"
                            } else {
                                "Comment"
                            }),
                    )
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .text_size(px(11.))
                            .child(comment.target.status(a).label()),
                    )
                    .child(crate::render::selectable(
                        &comment.id,
                        "body",
                        comment.body.clone(),
                        TextMode {
                            all: false,
                            components: false,
                        },
                    ));
                if let Target::Text { quote, .. } = &target {
                    card = card.child(div().text_color(rgb(MUTED)).child(format!("“{quote}”")));
                }
                let locate_id = id.clone();
                card = card.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            review_button_dynamic(format!("locate:{locate_id}"), "Select target")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this
                                        .artifact
                                        .as_ref()
                                        .is_some_and(|a| a.find(target.node_id()).is_some())
                                    {
                                        this.selected_id = Some(target.node_id().into());
                                        this.component_mode = true;
                                        this.status = "Target outlined in the document".into();
                                        cx.notify();
                                    }
                                })),
                        )
                        .child(
                            review_button_dynamic(
                                format!("resolve:{id}"),
                                if comment.resolved {
                                    "Reopen"
                                } else {
                                    "Resolve"
                                },
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.change_review("resolve", Some(id.clone()), cx)
                                },
                            )),
                        ),
                );
                panel = panel.child(card);
            }
            if !self.review.changes.is_empty() {
                panel = panel.child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Changes since last review"),
                );
                for change in &self.review.changes {
                    let mut card = div()
                        .p_3()
                        .bg(rgb(BACKGROUND))
                        .rounded_md()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(format!(
                            "{:?} {} · {}",
                            change.kind,
                            change.node_type,
                            change
                                .after
                                .as_ref()
                                .or(change.before.as_ref())
                                .and_then(|v| v
                                    .get("label")
                                    .or_else(|| v.get("title"))
                                    .or_else(|| v.get("text")))
                                .and_then(|v| v.as_str())
                                .unwrap_or(&change.node_id)
                        ));
                    if change.kind == ChangeKind::Changed {
                        let before = change.before.as_ref().and_then(|v| v.as_object());
                        let after = change.after.as_ref().and_then(|v| v.as_object());
                        if let (Some(before), Some(after)) = (before, after) {
                            for (field, value) in after {
                                if before.get(field) != Some(value) {
                                    card = card.child(div().text_size(px(11.)).child(format!(
                                            "{field}: {} → {value}",
                                            before
                                                .get(field)
                                                .map(|v| v.to_string())
                                                .unwrap_or_else(|| "—".into())
                                        )));
                                }
                            }
                        }
                    }
                    panel = panel.child(card);
                }
                panel = panel.child(
                    review_button("accept-revision", "Mark this revision reviewed").on_click(
                        cx.listener(|this, _, _, cx| this.change_review("accept", None, cx)),
                    ),
                );
            }
        }
        panel.into_any_element()
    }
}
fn review_button(id: &'static str, label: &str) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(BACKGROUND)))
        .child(label.to_owned())
}
fn review_button_dynamic(id: String, label: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .child(label.to_owned())
}

/// Preserve the painted text selection before GPUI Kit starts a new pointer
/// gesture. The outer root decoration runs before its selection layer clears it.
pub struct ReviewSelection;
impl Render for ReviewSelection {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
impl gpui_base::RootPlugin for ReviewSelection {
    fn decorate(
        &self,
        surface: AnyElement,
        root: &gpui_base::Root,
        _: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        let view = root.view().clone();
        div()
            .size_full()
            .capture_any_mouse_down(move |_, window, cx| {
                let quote = gpui_base::TextSelection::selected_text(window, cx);
                if let Ok(preview) = view.clone().downcast::<Preview>() {
                    preview.update(cx, |p, _| p.capture_review_quote(quote));
                } else if let Ok(workspace) = view.clone().downcast::<crate::workspace::Workspace>()
                {
                    workspace.update(cx, |w, cx| w.capture_review_quote(quote, cx));
                }
            })
            .child(surface)
    }
}
impl Preview {
    pub fn capture_review_quote(&mut self, quote: String) {
        if !self.component_mode && self.review.draft.is_none() {
            self.review.quote = quote;
        }
    }
}
