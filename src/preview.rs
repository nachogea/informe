use crate::{
    artifact::Artifact,
    loader::{FileLoader, LoadResult, POLL_INTERVAL},
    render::*,
};
use gpui::{
    AnyElement, ClipboardItem, Context, Focusable, FontWeight, Task, Window, div, prelude::*, px,
    rgb,
};
use std::path::PathBuf;

pub fn advanced_tools_enabled() -> bool {
    // The test UI still exercises legacy selection and review transactions.
    #[cfg(test)]
    {
        true
    }
    #[cfg(not(test))]
    {
        std::env::var_os("INFORME_EXPERIMENTAL_TOOLS").is_some()
            || std::env::var_os("ARTIFACT_PREVIEW_EXPERIMENTAL_TOOLS").is_some()
    }
}

pub struct Preview {
    pub path: PathBuf,
    pub artifact: Option<Artifact>,
    pub selected_id: Option<String>,
    pub review: crate::review_panel::ReviewUi,
    pub viewer: crate::viewer::ViewerUi,
    error: Option<String>,
    pub(crate) status: String,
    pub(crate) revision: usize,
    force_reload: bool,
    reload_task: Option<Task<()>>,
    pub ready_file: Option<PathBuf>,
    marked_revision: usize,
    pub focus: Option<gpui::FocusHandle>,
    pub focus_requested: bool,
    pub component_mode: bool,
    pub select_all: bool,
    show_inspector: bool,
    text_revision: usize,
    open_after_revision: usize,
    pub open_replies: Vec<std::sync::mpsc::Sender<Result<(), String>>>,
}

impl Preview {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut preview = Self {
            path: path.clone(),
            artifact: None,
            selected_id: None,
            viewer: crate::viewer::ViewerUi::default(),
            review: crate::review_panel::ReviewUi {
                show_changes: false,
                ..Default::default()
            },
            error: None,
            status: "Loading artifact…".into(),
            revision: 0,
            force_reload: false,
            reload_task: None,
            ready_file: None,
            marked_revision: 0,
            focus: Some(cx.focus_handle()),
            focus_requested: true,
            component_mode: false,
            select_all: false,
            show_inspector: false,
            text_revision: 0,
            open_after_revision: 0,
            open_replies: Vec::new(),
        };
        preview.reload_task = Some(cx.spawn(async move |entity, cx| {
            let mut loader = FileLoader::default();
            let mut last_artifact = None;
            let mut review_initialized = false;
            loop {
                let Ok((force, isolated, reviewing)) = entity.update(cx, |this, _| {
                    (
                        std::mem::take(&mut this.force_reload),
                        this.ready_file.is_some(),
                        this.review.show || this.review.draft.is_some(),
                    )
                }) else {
                    break;
                };
                let read_path = path.clone();
                // Only this loop loads files. Reads and parsing never block rendering,
                // and serialized loads cannot apply results out of order.
                let (next_loader, result, next_artifact, initialized, review) = cx
                    .background_executor()
                    .spawn(async move {
                        let result = loader.poll(&read_path, force);
                        if let Some(ref result) = result
                            && let Ok(ref artifact) = result.artifact
                        {
                            last_artifact = Some(artifact.clone());
                        }
                        let review = last_artifact
                            .as_ref()
                            .filter(|_| !isolated && reviewing)
                            .map(|artifact| {
                                if !review_initialized {
                                    if !crate::review::sidecar(&read_path).exists() {
                                        crate::review::edit(&read_path, None, |_, _| Ok(()))?;
                                    }
                                    review_initialized = true;
                                }
                                if !crate::review::sidecar(&read_path).exists() {
                                    return Err(
                                    "Review sidecar disappeared; restore it to continue reviewing"
                                        .into(),
                                );
                                }
                                crate::review::load(&read_path, artifact)
                            });
                        (loader, result, last_artifact, review_initialized, review)
                    })
                    .await;
                loader = next_loader;
                last_artifact = next_artifact;
                review_initialized = initialized;
                if entity
                    .update(cx, |this, cx| {
                        let loaded = result.is_some();
                        if let Some(result) = result {
                            this.apply_load(result);
                        }
                        let reviewed = review.is_some_and(|result| this.apply_review(result));
                        if loaded || reviewed {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
                cx.background_executor().timer(POLL_INTERVAL).await;
            }
        }));
        preview
    }

    fn apply_load(&mut self, result: LoadResult) {
        match result.artifact {
            Ok(artifact) => {
                artifact.reconcile_selection(&mut self.selected_id);
                self.viewer.rebuild(&artifact);
                self.artifact = Some(artifact);
                self.refresh_changes();
                self.error = None;
                self.revision += 1;
                self.status = format!("Updated · revision {}", self.revision);
                let _load_duration = result.elapsed;
            }
            Err(error) => {
                for reply in self.open_replies.drain(..) {
                    let _ = reply.send(Err(error.clone()));
                }
                self.error = Some(error);
                self.status = if self.artifact.is_some() {
                    "Showing last valid artifact"
                } else {
                    "Waiting for a valid artifact"
                }
                .into();
            }
        }
    }

    fn copy_selected_text(&mut self, cx: &mut Context<Self>) {
        if let Some(node) = self
            .selected_id
            .as_deref()
            .and_then(|id| self.artifact.as_ref()?.find(id))
        {
            cx.write_to_clipboard(ClipboardItem::new_string(node.text_content()));
            self.status = format!("Copied text from {}", node.id());
            cx.notify();
        }
    }

    pub fn title(&self) -> String {
        fn heading(n: &crate::artifact::ArtifactNode) -> Option<&str> {
            if let crate::artifact::ArtifactNode::Heading { text, .. } = n {
                return Some(text);
            }
            n.children().iter().find_map(heading)
        }
        self.artifact
            .as_ref()
            .and_then(|a| heading(&a.root))
            .map(str::to_owned)
            .unwrap_or_else(|| {
                self.path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
    }
    pub fn request_open(
        &mut self,
        reply: std::sync::mpsc::Sender<Result<(), String>>,
        cx: &mut Context<Self>,
    ) {
        self.focus_requested = true;
        self.open_after_revision = self.revision + 1;
        self.open_replies.push(reply);
        self.force_reload = true;
        cx.notify();
    }
    fn selected_node(&self) -> Option<&crate::artifact::ArtifactNode> {
        self.selected_id
            .as_deref()
            .and_then(|id| self.artifact.as_ref()?.find(id))
    }
    fn export(&mut self, save: bool, component: bool, cx: &mut Context<Self>) {
        let Some(node) = (if component && self.component_mode {
            self.selected_node()
        } else {
            self.artifact.as_ref().map(|a| &a.root)
        })
        .cloned() else {
            self.status = "Select a component first".into();
            cx.notify();
            return;
        };
        let picker = if save {
            Some(cx.prompt_for_new_path(
                self.path.parent().unwrap_or(std::path::Path::new(".")),
                Some(&format!(
                    "{}.png",
                    self.path.file_stem().unwrap_or_default().to_string_lossy()
                )),
            ))
        } else {
            None
        };
        self.status = "Preparing image…".into();
        cx.notify();
        cx.spawn(async move |entity, cx| {
            let path = if let Some(picker) = picker {
                match picker.await {
                    Ok(Ok(Some(path))) => Some(path),
                    Ok(Ok(None)) => {
                        let _ = entity.update(cx, |this, cx| {
                            this.status = "Export cancelled".into();
                            cx.notify();
                        });
                        return;
                    }
                    _ => {
                        let _ = entity.update(cx, |this, cx| {
                            this.status = "Could not open save dialog".into();
                            cx.notify();
                        });
                        return;
                    }
                }
            } else {
                None
            };
            let result = cx
                .background_executor()
                .spawn(async move {
                    let bytes = crate::export::png(&node)?;
                    if let Some(path) = path {
                        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                        Ok::<_, String>((None, format!("Saved {}", path.display())))
                    } else {
                        Ok((Some(bytes), "Copied image".into()))
                    }
                })
                .await;
            let _ = entity.update(cx, |this, cx| {
                match result {
                    Ok((bytes, status)) => {
                        if let Some(bytes) = bytes {
                            cx.write_to_clipboard(ClipboardItem::new_image(
                                &gpui::Image::from_bytes(gpui::ImageFormat::Png, bytes),
                            ));
                        }
                        this.status = status;
                    }
                    Err(e) => this.status = format!("Export failed: {e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub fn copy_table(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(text) = self
            .artifact
            .as_ref()
            .and_then(|artifact| artifact.find(id))
            .and_then(|node| crate::export::table_data(node, false).ok())
        else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.status = "Copied table as TSV".into();
        cx.notify();
    }

    pub fn save_table(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(text) = self
            .artifact
            .as_ref()
            .and_then(|artifact| artifact.find(id))
            .and_then(|node| crate::export::table_data(node, true).ok())
        else {
            return;
        };
        let picker = cx.prompt_for_new_path(
            self.path.parent().unwrap_or(std::path::Path::new(".")),
            Some("table.csv"),
        );
        cx.spawn(async move |entity, cx| {
            let status = match picker.await {
                Ok(Ok(Some(path))) => {
                    cx.background_executor()
                        .spawn(async move {
                            std::fs::write(&path, text)
                                .map(|_| format!("Saved {}", path.display()))
                                .unwrap_or_else(|e| format!("Export failed: {e}"))
                        })
                        .await
                }
                Ok(Ok(None)) => "Export cancelled".into(),
                _ => "Could not open save dialog".into(),
            };
            let _ = entity.update(cx, |this, cx| {
                this.status = status;
                cx.notify();
            });
        })
        .detach();
    }

    fn inspector(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div()
            .w(px(260.))
            .flex_shrink_0()
            .h_full()
            .p_6()
            .bg(rgb(PAPER))
            .border_l_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Selected node"),
            );
        let node = self
            .selected_id
            .as_deref()
            .and_then(|id| self.artifact.as_ref()?.find(id));
        if let Some(node) = node {
            panel = panel
                .child(metadata("ID", node.id()))
                .child(metadata("TYPE", node.node_type()));
            if node.node_type() == "button" {
                panel = panel.child(metadata("ACTION", "noop"));
            }
            if !node.children().is_empty() {
                panel = panel.child(metadata("CHILDREN", &node.children().len().to_string()));
            }
            if !node.text_content().is_empty() {
                panel = panel.child(
                    div()
                        .id("copy-text")
                        .debug_selector(|| "copy-text".into())
                        .px_4()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(ACCENT))
                        .text_color(rgb(PAPER))
                        .cursor_pointer()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .hover(|style| style.bg(rgb(0x4557c3)))
                        .on_click(cx.listener(|this, _, _, cx| this.copy_selected_text(cx)))
                        .child("Copy text"),
                );
            }
        } else {
            panel = panel.child(
                div()
                    .text_size(px(14.))
                    .text_color(rgb(MUTED))
                    .child("Click an artifact component to inspect its semantic identity."),
            );
        }
        panel.into_any_element()
    }
}

fn tool(id: &'static str, label: &str) -> gpui::Stateful<gpui::Div> {
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

fn metadata(label: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(rgb(INK))
                .child(value.to_owned()),
        )
}

impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare_markdown(cx);
        if self.markdown_ready(cx)
            && self.revision > self.marked_revision
            && self.error.is_none()
            && let Some(path) = self.ready_file.clone()
        {
            let revision = self.revision;
            self.marked_revision = revision;
            // GPUI invokes this after the artifact frame is rendered. This is
            // a framework marker, not proof of physical screen presentation.
            window.on_next_frame(move |_, _| {
                let bytes = serde_json::to_vec(&serde_json::json!({
                    "revision": revision, "marker": "gpui_after_frame",
                }))
                .expect("marker JSON");
                let temporary = path.with_extension("pending");
                if let Err(error) = std::fs::write(&temporary, bytes)
                    .and_then(|_| std::fs::rename(&temporary, &path))
                {
                    eprintln!("Readiness marker: {error}");
                }
            });
        }
        if self.text_revision != self.revision {
            if let Some(query) = &self.viewer.query {
                self.viewer.matches = self.viewer.search(&query.read(cx).value());
                self.viewer.current = 0;
            }
            gpui_base::TextSelection::clear(window, cx);
            self.select_all = false;
            self.review.quote.clear();
            self.text_revision = self.revision;
        }
        if self.error.is_none()
            && self.artifact.is_some()
            && self.markdown_ready(cx)
            && self.revision >= self.open_after_revision
            && !self.open_replies.is_empty()
        {
            let replies = std::mem::take(&mut self.open_replies);
            window.on_next_frame(move |window, cx| {
                window.activate_window();
                cx.activate(true);
                for reply in replies {
                    let _ = reply.send(Ok(()));
                }
            });
        }
        if self.focus_requested {
            if let Some(focus) = &self.focus {
                window.focus(focus, cx);
            }
            self.focus_requested = false;
        }
        self.viewer.width = f32::from(window.viewport_size().width);
        let toolbar = div()
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(px(44.))
            .px_5()
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(BORDER))
            .flex_shrink_0()
            .child(
                crate::viewer::button("outline-toggle", "Contents").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.viewer.show_outline = !this.viewer.show_outline;
                        this.viewer.menu = false;
                        cx.notify();
                    },
                )),
            )
            .child(div().flex_1())
            .child(
                crate::viewer::button("find-document", "Find")
                    .on_click(cx.listener(|this, _, window, cx| this.start_find(window, cx))),
            )
            .child(
                crate::viewer::button("more-tools", "More ···").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.viewer.menu = !this.viewer.menu;
                        cx.notify();
                    },
                )),
            )
            .when(self.viewer.menu, |d| {
                d.child(
                    gpui::deferred(
                        div()
                            .absolute()
                            .right_4()
                            .top(px(42.))
                            .w(px(215.))
                            .occlude()
                            .p_2()
                            .bg(rgb(PAPER))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .rounded_lg()
                            .shadow_md()
                            .flex()
                            .flex_col()
                            .child(
                                crate::viewer::button("copy-document", "Copy document text")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(a) = &this.artifact {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                a.root.text_content(),
                                            ));
                                            this.status = "Copied document text".into();
                                        }
                                        this.viewer.menu = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                crate::viewer::button("copy-image", "Copy image (PNG)").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.viewer.menu = false;
                                        this.export(false, true, cx);
                                    }),
                                ),
                            )
                            .child(crate::viewer::button("save-image", "Export PNG…").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.viewer.menu = false;
                                    this.export(true, false, cx);
                                }),
                            ))
                            .when(advanced_tools_enabled(), |d| {
                                d.child(
                                    crate::viewer::button(
                                        "component-mode",
                                        if self.component_mode {
                                            "Done selecting components"
                                        } else {
                                            "Select components"
                                        },
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            this.component_mode = !this.component_mode;
                                            this.viewer.menu = false;
                                            this.select_all = false;
                                            gpui_base::TextSelection::clear(window, cx);
                                            cx.notify();
                                        },
                                    )),
                                )
                            })
                            .when(advanced_tools_enabled(), |d| {
                                d.child(
                                    crate::viewer::button("review-toggle", "Review / comments")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.review.show = !this.review.show;
                                            this.viewer.menu = false;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(
                                self.artifact
                                    .as_ref()
                                    .and_then(|a| a.metadata.as_ref())
                                    .is_some(),
                                |d| {
                                    d.child(
                                        crate::viewer::button("sources-toggle", "Document info")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.viewer.sources = !this.viewer.sources;
                                                this.viewer.menu = false;
                                                cx.notify();
                                            })),
                                    )
                                },
                            )
                            .when(advanced_tools_enabled(), |d| {
                                d.child(crate::viewer::button("reload", "Reload").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.force_reload = true;
                                        this.viewer.menu = false;
                                        cx.notify();
                                    }),
                                ))
                            })
                            .when(advanced_tools_enabled(), |d| {
                                d.child(
                                    crate::viewer::button("inspect-toggle", "Developer inspector")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.show_inspector = !this.show_inspector;
                                            this.viewer.menu = false;
                                            cx.notify();
                                        })),
                                )
                            }),
                    )
                    .with_priority(2),
                )
            });
        let mut component_tools = div()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .py_2()
            .text_size(px(12.))
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(BORDER));
        if let Some(node) = self.selected_node() {
            component_tools = component_tools.child(format!("Selected {}", node.node_type()));
        }
        if self.component_mode && self.selected_id.is_some() {
            component_tools = component_tools.child(
                tool("copy-component", "Copy component text")
                    .on_click(cx.listener(|this, _, _, cx| this.copy_selected_text(cx))),
            );
            if matches!(
                self.selected_node(),
                Some(crate::artifact::ArtifactNode::Table { .. })
            ) {
                component_tools = component_tools
                    .child(tool("copy-table", "Copy table").on_click(cx.listener(
                        |this, _, _, cx| {
                            if let Some(node) = this.selected_node() {
                                match crate::export::table_data(node, false) {
                                    Ok(text) => {
                                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                                        this.status = "Copied table as TSV".into();
                                    }
                                    Err(e) => this.status = e,
                                }
                                cx.notify();
                            }
                        },
                    )))
                    .child(tool("save-table", "Save CSV…").on_click(cx.listener(
                        |this, _, _, cx| {
                            if let Some(id) = this.selected_id.clone() {
                                this.save_table(&id, cx);
                            }
                        },
                    )));
            }
        }

        let toolbar = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .child(toolbar)
            .when(self.review.show, |d| d.child(self.review_strip(cx)))
            .when(self.component_mode && self.selected_id.is_some(), |d| {
                d.child(component_tools)
            });

        let dashboard = self.artifact.as_ref().is_some_and(|a| has_data(&a.root));
        let outline_visible = self.viewer.show_outline && self.viewer.width >= 760.;
        let content = div()
            .id("artifact-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_scroll()
            .track_scroll(&self.viewer.scroll)
            .bg(rgb(if dashboard { BACKGROUND } else { PAPER }))
            .child(
                div()
                    .px(px(if self.viewer.width < 700. { 24. } else { 48. }))
                    .py_8()
                    .mx_auto()
                    .w_full()
                    .max_w(px(if dashboard { 1120. } else { 820. }))
                    .child(match &self.artifact {
                        Some(artifact) => render_node(
                            &artifact.root,
                            if self.component_mode {
                                self.selected_id.as_deref()
                            } else {
                                None
                            },
                            TextMode {
                                all: self.select_all,
                                components: self.component_mode,
                            },
                            (self.viewer.width
                                - if outline_visible { 196. } else { 0. }
                                - if self.review.show { 340. } else { 0. }
                                - 96.)
                                .max(240.),
                            &self.viewer,
                            if self.review.show && self.review.show_changes {
                                &self.review.changes
                            } else {
                                &[]
                            },
                            cx,
                        ),
                        None => div()
                            .text_color(rgb(MUTED))
                            .child("Waiting for artifact JSON…")
                            .into_any_element(),
                    }),
            );
        let mut body = div().flex_1().min_h_0().flex().relative();
        if outline_visible {
            body = body.child(self.navigation(cx));
        }
        body = body.child(content);
        if self.viewer.show_outline && !outline_visible {
            body = body.child(
                gpui::deferred(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .h_full()
                        .w(px(196.))
                        .occlude()
                        .shadow_lg()
                        .child(self.navigation(cx)),
                )
                .with_priority(1),
            );
        }
        if self.review.show {
            body = body.child(self.review_panel(cx));
        }
        if self.show_inspector {
            body = body.child(self.inspector(cx));
        }

        let mut root =
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(BACKGROUND))
                .text_color(rgb(INK))
                .font_family(".AppleSystemUIFont")
                .when_some(self.focus.as_ref(), |d, focus| d.track_focus(focus))
                .on_action(cx.listener(|this, _: &gpui_base::input::Copy, window, cx| {
                    if this.select_all {
                        if let Some(a) = &this.artifact {
                            cx.write_to_clipboard(ClipboardItem::new_string(a.root.text_content()));
                        }
                    } else {
                        let text = gpui_base::TextSelection::selected_text(window, cx);
                        if text.is_empty() {
                            cx.propagate();
                        } else {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                        }
                    }
                }))
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.viewer.find = false;
                        this.viewer.menu = false;
                        this.focus_requested = true;
                        cx.notify();
                        return;
                    }
                    if event.keystroke.modifiers.platform {
                        if event.keystroke.key == "f" {
                            this.start_find(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if advanced_tools_enabled()
                            && event.keystroke.key == "m"
                            && event.keystroke.modifiers.shift
                        {
                            if !this.component_mode {
                                this.review.quote =
                                    gpui_base::TextSelection::selected_text(window, cx);
                            }
                            this.begin_comment(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if this
                            .viewer
                            .query
                            .as_ref()
                            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window))
                        {
                            return;
                        }
                        if this.review.editor.as_ref().is_some_and(|editor| {
                            editor.read(cx).focus_handle(cx).is_focused(window)
                        }) {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "a" => {
                                gpui_base::TextSelection::clear(window, cx);
                                this.select_all = true;
                                for (_, state) in this.viewer.markdown.values() {
                                    state.update(cx, |s, cx| s.select_all(cx));
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "c" if this.select_all => {
                                if let Some(a) = &this.artifact {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        a.root.text_content(),
                                    ));
                                }
                                cx.stop_propagation();
                            }
                            _ => {}
                        }
                    }
                }))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if this.select_all {
                            this.select_all = false;
                            cx.notify();
                        }
                    }),
                )
                .child(toolbar);
        if self.viewer.find {
            root = root.child(self.find_bar(cx));
        }
        if self.viewer.sources
            && let Some(a) = &self.artifact
        {
            let provenance = a
                .metadata
                .as_ref()
                .map(|m| {
                    if m.sources.is_empty() {
                        return "Sources not supplied".to_owned();
                    }
                    format!(
                        "Sources supplied by artifact: {}",
                        m.sources
                            .iter()
                            .map(|s| format!(
                                "{}{}{}",
                                s.label,
                                s.uri
                                    .as_ref()
                                    .map(|u| format!(" ({u})"))
                                    .unwrap_or_default(),
                                s.as_of
                                    .as_ref()
                                    .map(|t| format!(" · as of {t}"))
                                    .unwrap_or_default()
                            ))
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                })
                .unwrap_or_else(|| "Sources not supplied".into());
            root = root.child(
                div()
                    .flex_shrink_0()
                    .px_6()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(MUTED))
                    .child(provenance),
            );
        }
        if let Some(error) = &self.error {
            root = root.child(
                div()
                    .flex_shrink_0()
                    .px_6()
                    .py_3()
                    .bg(rgb(0xffeeee))
                    .text_color(rgb(0xb74b4b))
                    .text_size(px(12.))
                    .child(error.clone()),
            );
        }
        root.child(body).when(
            self.component_mode
                || self.error.is_some()
                || self.status.starts_with("Copied")
                || self.status.contains("saved")
                || self.status.contains("Export"),
            |root| {
                root.child(
                    div()
                        .flex_shrink_0()
                        .px_6()
                        .py_2()
                        .border_t_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .gap_4()
                        .text_size(px(11.))
                        .text_color(rgb(MUTED))
                        .child(div().flex_1().child(self.status.clone()))
                        .child(if self.component_mode {
                            "Click a component · Copy image uses selection"
                        } else {
                            "Drag to select text · Copy image uses document"
                        }),
                )
            },
        )
    }
}

fn has_data(node: &crate::artifact::ArtifactNode) -> bool {
    matches!(
        node,
        crate::artifact::ArtifactNode::Metric { .. }
            | crate::artifact::ArtifactNode::Chart { .. }
            | crate::artifact::ArtifactNode::Table { .. }
    ) || node.children().iter().any(has_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn nonvirtualized_markdown_can_acknowledge_ready(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        let _ = cx.new(|cx| {
            let mut v = Preview::new(PathBuf::from("/nonexistent-ready-test.md"), cx);
            v.reload_task = None;
            v.artifact = Some(
                Artifact::parse_document(
                    b"# Plan\n\nRead **this**.",
                    std::path::Path::new("plan.md"),
                )
                .unwrap(),
            );
            v.revision = 1;
            v.prepare_markdown(cx);
            assert!(v.markdown_ready(cx));
            for (_, state) in v.viewer.markdown.values() {
                assert_eq!(state.read(cx).list_state().item_count(), 0);
                assert!(state.read(cx).rendered_text().as_str().contains("this"));
            }
            v
        });
    }

    #[gpui::test]
    fn markdown_without_visible_text_can_acknowledge_ready(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        for source in [
            "",
            "---\n",
            "```\n```\n",
            "# Report\n\n---\n\n```\n```\n\nDone.",
        ] {
            let _ = cx.new(|cx| {
                let mut v = Preview::new(PathBuf::from("/nonexistent-empty-test.md"), cx);
                v.reload_task = None;
                v.artifact = Some(
                    Artifact::parse_document(source.as_bytes(), std::path::Path::new("report.md"))
                        .unwrap(),
                );
                v.revision = 1;
                v.prepare_markdown(cx);
                assert!(v.markdown_ready(cx), "source: {source:?}");
                v
            });
        }
    }

    #[gpui::test]
    fn large_empty_markdown_waits_for_parsing_before_acknowledging_ready(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_base::init);
        let view = cx.new(|cx| {
            let mut v = Preview::new(PathBuf::from("/nonexistent-large-test.json"), cx);
            v.reload_task = None;
            v.artifact = Some(Artifact {
                version: 1,
                metadata: None,
                root: crate::artifact::ArtifactNode::Markdown {
                    id: "empty".into(),
                    source: " ".repeat(8192),
                },
            });
            v.revision = 1;
            v.prepare_markdown(cx);
            assert!(!v.markdown_ready(cx));
            v
        });
        cx.run_until_parked();
        view.update(cx, |v, cx| assert!(v.markdown_ready(cx)));
    }

    #[gpui::test]
    fn chart_and_table_render_and_select_as_semantic_nodes(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        let artifact = Artifact::parse(br#"{"version":1,"root":{"type":"column","id":"root","children":[{"type":"chart","id":"chart","title":"Zero baseline","kind":"bar","data":[{"label":"A","value":0}]},{"type":"table","id":"table","title":"Results","columns":["Name","Value"],"rows":[["A","0"]]}]}}"#).unwrap();
        let (view, cx) = cx.add_window_view(|_, _| Preview {
            path: PathBuf::from("test.json"),
            artifact: Some(artifact),
            selected_id: None,
            viewer: crate::viewer::ViewerUi::default(),
            review: crate::review_panel::ReviewUi {
                show_changes: false,
                ..Default::default()
            },
            error: None,
            status: String::new(),
            revision: 1,
            force_reload: false,
            reload_task: None,
            ready_file: None,
            marked_revision: 0,
            focus: None,
            focus_requested: false,
            component_mode: true,
            select_all: false,
            show_inspector: true,
            text_revision: 0,
            open_after_revision: 0,
            open_replies: Vec::new(),
        });
        for id in ["chart", "table"] {
            let bounds = cx.debug_bounds(id).expect("data component rendered");
            assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            assert_eq!(
                view.read_with(cx, |view, _| view.selected_id.clone())
                    .as_deref(),
                Some(id)
            );
        }
        let copy = cx.debug_bounds("copy-text").expect("copy button rendered");
        cx.simulate_click(copy.center(), gpui::Modifiers::default());
        let clipboard = cx.update(|_, app| app.read_from_clipboard().unwrap().text().unwrap());
        assert_eq!(clipboard, "Results\nName\tValue\nA\t0");
        view.update(cx, |view, cx| view.copy_table("table", cx));
        let clipboard = cx.update(|_, app| app.read_from_clipboard().unwrap().text().unwrap());
        assert_eq!(clipboard, "Name\tValue\r\nA\t0\r\n");
    }

    #[gpui::test]
    fn nested_click_selects_deepest_node_and_container_whitespace(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        let artifact = Artifact::parse(br#"{"version":1,"root":{"type":"card","id":"card","children":[{"type":"button","id":"button","label":"Select me","action":{"type":"noop"}}]}}"#).unwrap();
        let (view, cx) = cx.add_window_view(|_, _| Preview {
            path: PathBuf::from("test.json"),
            artifact: Some(artifact),
            selected_id: None,
            viewer: crate::viewer::ViewerUi::default(),
            review: crate::review_panel::ReviewUi {
                show_changes: false,
                ..Default::default()
            },
            error: None,
            status: String::new(),
            revision: 1,
            force_reload: false,
            reload_task: None,
            ready_file: None,
            marked_revision: 0,
            focus: None,
            focus_requested: false,
            component_mode: true,
            select_all: false,
            show_inspector: true,
            text_revision: 0,
            open_after_revision: 0,
            open_replies: Vec::new(),
        });
        let button = cx.debug_bounds("button").expect("button rendered");
        cx.simulate_click(button.center(), gpui::Modifiers::default());
        assert_eq!(
            view.read_with(cx, |view, _| view.selected_id.clone())
                .as_deref(),
            Some("button")
        );
        let card = cx.debug_bounds("card").expect("card rendered");
        cx.simulate_click(
            card.origin + gpui::point(px(8.), px(8.)),
            gpui::Modifiers::default(),
        );
        assert_eq!(
            view.read_with(cx, |view, _| view.selected_id.clone())
                .as_deref(),
            Some("card")
        );
    }
    #[gpui::test]
    fn ordinary_drag_and_command_all_copy_without_inspector(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        cx.update(|cx| {
            gpui_base::Root::register_plugin::<crate::review_panel::ReviewSelection>(cx, |_, _| {
                crate::review_panel::ReviewSelection
            })
        });
        let mut preview = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                let mut v = Preview::new(PathBuf::from("/nonexistent-test.json"), cx);
                v.reload_task = None;
                v.artifact = Some(
                    Artifact::parse(
                        br#"{"version":1,"root":{"type":"text","id":"line","text":"alpha beta"}}"#,
                    )
                    .unwrap(),
                );
                v.revision = 1;
                v
            });
            preview = Some(view.clone());
            gpui_base::Root::new(view, window, cx)
        });
        let bounds = cx.debug_bounds("line").unwrap();
        let start = bounds.origin + gpui::point(px(5.), px(10.));
        let end = start + gpui::point(px(150.), px(0.));
        cx.simulate_mouse_down(start, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_move(
            end,
            Some(gpui::MouseButton::Left),
            gpui::Modifiers::default(),
        );
        cx.simulate_mouse_up(end, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.update(|window, app| {
            let _ = window.draw(app);
            assert_eq!(
                gpui_base::TextSelection::selected_text(window, app),
                "alpha beta"
            );
        });
        assert!(
            preview
                .as_ref()
                .unwrap()
                .read_with(cx, |v, _| v.selected_id.is_none())
        );
        cx.simulate_keystrokes("cmd-c");
        assert_eq!(
            cx.update(|_, app| app.read_from_clipboard().unwrap().text().unwrap()),
            "alpha beta"
        );
        let position = start + gpui::point(px(44.), px(0.));
        cx.simulate_event(gpui::MouseDownEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: gpui::MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(gpui::MouseUpEvent {
            position,
            modifiers: gpui::Modifiers::default(),
            button: gpui::MouseButton::Left,
            click_count: 2,
        });
        cx.update(|window, app| {
            let _ = window.draw(app);
            assert_eq!(
                gpui_base::TextSelection::selected_text(window, app),
                "alpha"
            );
        });
        cx.simulate_keystrokes("cmd-shift-m");
        // Keyboard commenting preserves the precise current selection.
        preview
            .as_ref()
            .unwrap()
            .read_with(cx, |v, _| match &v.review.draft.as_ref().unwrap().0 {
                crate::review::Target::Text { quote, .. } => assert_eq!(quote, "alpha"),
                _ => panic!("Selected text must become a quoted-text target"),
            });
        let cancel = cx.debug_bounds("cancel-comment").unwrap();
        cx.simulate_click(cancel.center(), gpui::Modifiers::default());
        cx.simulate_keystrokes("cmd-a cmd-c");
        assert!(preview.as_ref().unwrap().read_with(cx, |v, _| v.select_all));
        assert_eq!(
            cx.update(|_, app| app.read_from_clipboard().unwrap().text().unwrap()),
            "alpha beta"
        );
        preview.as_ref().unwrap().update(cx, |view, cx| {
            view.component_mode = true;
            view.select_all = false;
            cx.notify();
        });
        cx.update(gpui_base::TextSelection::clear);
        let bounds = cx.debug_bounds("line").unwrap();
        cx.simulate_click(bounds.center(), gpui::Modifiers::default());
        cx.update(|window, app| {
            let _ = window.draw(app);
            assert!(!gpui_base::TextSelection::has_selection(window, app));
        });
        assert_eq!(
            preview
                .as_ref()
                .unwrap()
                .read_with(cx, |v, _| v.selected_id.clone())
                .as_deref(),
            Some("line")
        );
    }

    #[test]
    fn invalid_reload_keeps_last_valid_artifact_and_selection() {
        let mut preview = Preview {
            path: PathBuf::from("test.json"),
            artifact: None,
            selected_id: None,
            viewer: crate::viewer::ViewerUi::default(),
            review: crate::review_panel::ReviewUi {
                show_changes: false,
                ..Default::default()
            },
            error: None,
            status: String::new(),
            revision: 0,
            force_reload: false,
            reload_task: None,
            ready_file: None,
            marked_revision: 0,
            focus: None,
            focus_requested: false,
            component_mode: true,
            select_all: false,
            show_inspector: true,
            text_revision: 0,
            open_after_revision: 0,
            open_replies: Vec::new(),
        };
        preview.apply_load(LoadResult {
            artifact: Artifact::parse(include_bytes!("../examples/dashboard.json")),
            elapsed: std::time::Duration::ZERO,
        });
        preview.selected_id = Some("projected-spend".into());
        preview.apply_load(LoadResult {
            artifact: Err("Invalid JSON".into()),
            elapsed: std::time::Duration::ZERO,
        });
        assert!(
            preview
                .artifact
                .as_ref()
                .unwrap()
                .find("projected-spend")
                .is_some()
        );
        assert_eq!(preview.selected_id.as_deref(), Some("projected-spend"));
        assert_eq!(preview.revision, 1);
        assert!(preview.error.is_some());
        preview.apply_load(LoadResult {
            artifact: Artifact::parse(br#"{"version":1,"root":{"type":"divider","id":"new"}}"#),
            elapsed: std::time::Duration::ZERO,
        });
        assert!(preview.selected_id.is_none());
        assert!(preview.error.is_none());
        assert_eq!(preview.revision, 2);
    }

    #[gpui::test]
    fn comment_editor_keeps_draft_when_artifact_changes(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_base::init);
        let path = std::env::temp_dir().join(format!("artifact-draft-{}.json", std::process::id()));
        let artifact = Artifact::parse(include_bytes!("../examples/review/cost-v1.json")).unwrap();
        std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
        let (view, cx) = cx.add_window_view(|_, _| Preview {
            path: path.clone(),
            artifact: Some(artifact),
            selected_id: Some("projected-spend".into()),
            viewer: crate::viewer::ViewerUi::default(),
            review: crate::review_panel::ReviewUi {
                show_changes: false,
                ..Default::default()
            },
            error: None,
            status: String::new(),
            revision: 1,
            force_reload: false,
            reload_task: None,
            ready_file: None,
            marked_revision: 0,
            focus: None,
            focus_requested: false,
            component_mode: true,
            select_all: false,
            show_inspector: false,
            text_revision: 0,
            open_after_revision: 0,
            open_replies: Vec::new(),
        });
        cx.update(|window, app| view.update(app, |preview, cx| preview.begin_comment(window, cx)));
        cx.simulate_input("Compare this against last month.");
        // A concurrent agent edit must not attach this draft to a newer snapshot.
        std::fs::write(&path, include_bytes!("../examples/review/cost-v2.json")).unwrap();
        let button = cx
            .debug_bounds("save-comment")
            .expect("comment composer rendered");
        cx.simulate_click(button.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |preview, app| {
            assert!(preview.review.draft.is_some());
            assert_eq!(
                preview
                    .review
                    .editor
                    .as_ref()
                    .unwrap()
                    .read(app)
                    .value()
                    .as_str(),
                "Compare this against last month."
            );
            assert!(
                preview.status.starts_with("Review not saved:"),
                "{}",
                preview.status
            );
        });
        assert!(!crate::review::sidecar(&path).exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(crate::review::sidecar(&path).with_extension("lock"));
    }
}
