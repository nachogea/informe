use crate::{
    CloseDocument, FindDocument, OpenDocument,
    ipc::{OpenRequest, legacy_runtime_dir, runtime_dir},
    preview::Preview,
    render::*,
};
use gpui::{prelude::*, *};
use std::{path::PathBuf, sync::mpsc, time::Duration};
fn load_recents() -> Vec<PathBuf> {
    // Existing installations retain their recent documents on the first launch.
    // New opens are persisted in Informe's own directory below.
    load_recents_from([runtime_dir(), legacy_runtime_dir()])
}
fn load_recents_from(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    dirs.into_iter()
        .find_map(|dir| {
            std::fs::read(dir.join("recent.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Vec<PathBuf>>(&bytes).ok())
        })
        .unwrap_or_default()
        .into_iter()
        .take(10)
        .collect()
}
pub struct Workspace {
    focus: FocusHandle,
    documents: Vec<Entity<Preview>>,
    active: usize,
    persist_recents: bool,
    recents: Vec<PathBuf>,
    _requests: Option<Task<()>>,
}
impl Workspace {
    pub fn new(
        path: Option<PathBuf>,
        requests: Option<mpsc::Receiver<OpenRequest>>,
        ready_file: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) -> Self {
        let recents = load_recents();
        let mut this = Self {
            focus: cx.focus_handle(),
            documents: Vec::new(),
            active: 0,
            persist_recents: ready_file.is_none(),
            recents,
            _requests: None,
        };
        if let Some(path) = path {
            this.open(path, None, cx);
            this.documents[0].update(cx, |v, _| v.ready_file = ready_file);
        }
        if let Some(requests) = requests {
            this._requests = Some(cx.spawn(async move |entity, cx| {
                loop {
                    while let Ok(request) = requests.try_recv() {
                        if entity
                            .update(cx, |this, cx| {
                                // A minimized/hidden window may not draw a frame until activated.
                                // Bring it forward before waiting for its render acknowledgment.
                                cx.activate(true);
                                for handle in cx.windows() {
                                    let _ = handle.update(cx, |_, window, _| {
                                        window.activate_window();
                                        window.refresh();
                                    });
                                }
                                this.open(request.path, Some(request.reply), cx)
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(20))
                        .await;
                    if entity.update(cx, |_, _| ()).is_err() {
                        return;
                    }
                }
            }));
        }
        this
    }
    fn open(
        &mut self,
        path: PathBuf,
        reply: Option<mpsc::Sender<Result<(), String>>>,
        cx: &mut Context<Self>,
    ) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = self.documents.iter().position(|d| d.read(cx).path == path) {
            self.active = index;
        } else {
            self.documents
                .push(cx.new(|cx| Preview::new(path.clone(), cx)));
            self.active = self.documents.len() - 1;
        }
        self.documents[self.active].update(cx, |v, _| v.focus_requested = true);
        if let Some(reply) = reply {
            self.documents[self.active].update(cx, |v, cx| v.request_open(reply, cx));
        }
        self.recents.retain(|p| p != &path);
        self.recents.insert(0, path);
        self.recents.truncate(10);
        // A tiny atomic metadata write is serialized here so older opens cannot
        // overwrite a newer recent-document list from background tasks.
        let dir = runtime_dir();
        if self.persist_recents && std::fs::create_dir_all(&dir).is_ok() {
            let temporary = dir.join("recent.pending");
            if let Ok(bytes) = serde_json::to_vec(&self.recents) {
                let _ = std::fs::write(&temporary, bytes)
                    .and_then(|_| std::fs::rename(&temporary, dir.join("recent.json")));
            }
        }
        cx.notify();
    }
    fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.documents.remove(index);
        self.active = if index < self.active {
            self.active - 1
        } else {
            self.active.min(self.documents.len().saturating_sub(1))
        };
        gpui_base::TextSelection::clear(window, cx);
        if let Some(view) = self.documents.get(self.active) {
            view.update(cx, |v, _| v.focus_requested = true);
        }
        cx.notify();
    }
    pub fn capture_review_quote(&mut self, quote: String, cx: &mut Context<Self>) {
        if let Some(preview) = self.documents.get(self.active) {
            preview.update(cx, |p, _| p.capture_review_quote(quote));
        }
    }
    fn pick(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open an artifact JSON document".into()),
        });
        cx.spawn(async move |entity, cx| {
            if let Ok(Ok(Some(paths))) = picker.await {
                for path in paths {
                    let _ = entity.update(cx, |this, cx| this.open(path, None, cx));
                }
            }
        })
        .detach();
    }
}
impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.documents.is_empty() {
            window.focus(&self.focus, cx);
        }
        window.set_window_title(
            &self
                .documents
                .get(self.active)
                .map(|v| format!("{} — Informe", v.read(cx).title()))
                .unwrap_or_else(|| "Informe".into()),
        );
        let mut tabs = div()
            .id("document-tabs")
            .w_full()
            .overflow_x_scroll()
            .flex()
            .items_center()
            .gap_2()
            .px_4()
            .py_1()
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(BORDER))
            .text_size(px(12.))
            .flex_shrink_0();
        for (index, view) in self.documents.iter().enumerate() {
            let title = view.read(cx).title();
            tabs = tabs.child(
                gpui_base::Button::new(("tab", index))
                    .accessibility_label(if index == self.active {
                        format!("Current document: {title}")
                    } else {
                        format!("Open document: {title}")
                    })
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .when(index == self.active, |d| d.bg(rgb(0xeef3ff)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.active = index;
                        gpui_base::TextSelection::clear(window, cx);
                        this.documents[index].update(cx, |v, _| {
                            v.select_all = false;
                            v.focus_requested = true;
                        });
                        cx.notify();
                    }))
                    .child(div().max_w(px(200.)).truncate().child(title.clone()))
                    .child(
                        gpui_base::Button::new(("close", index))
                            .accessibility_label(format!("Close document: {title}"))
                            .child("×")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.close(index, window, cx);
                            })),
                    ),
            );
        }
        tabs = tabs.child(
            gpui_base::Button::new("open-file")
                .accessibility_label("Open document")
                .flex_shrink_0()
                .px_3()
                .py_1()
                .cursor_pointer()
                .child("Open…")
                .on_click(cx.listener(|this, _, _, cx| this.pick(cx))),
        );
        let content = if let Some(view) = self.documents.get(self.active) {
            view.clone().into_any_element()
        } else {
            div()
                .p_8()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .id("welcome-heading")
                        .role(Role::Heading)
                        .aria_level(1)
                        .aria_label("Your reports, ready to read")
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .child("Your reports, ready to read"),
                )
                .child("Ask your agent for a report, or open a JSON or Markdown document.")
                .child(div().text_color(rgb(MUTED)).child("Recent documents"))
                .children(self.recents.iter().enumerate().map(|(i, path)| {
                    let path = path.clone();
                    let label = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    gpui_base::Button::new(("recent", i))
                        .accessibility_label(format!("Open recent document: {label}"))
                        .py_1()
                        .cursor_pointer()
                        .child(label)
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.open(path.clone(), None, cx)),
                        )
                }))
                .into_any_element()
        };
        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &OpenDocument, _, cx| this.pick(cx)))
            .on_action(cx.listener(|this, _: &CloseDocument, window, cx| {
                if !this.documents.is_empty() {
                    this.close(this.active, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FindDocument, window, cx| {
                if let Some(view) = this.documents.get(this.active) {
                    view.update(cx, |preview, cx| preview.start_find(window, cx));
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .font_family(".AppleSystemUIFont")
            .text_color(rgb(INK))
            .bg(rgb(BACKGROUND))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.platform {
                    match event.keystroke.key.as_str() {
                        "w" if !this.documents.is_empty() => {
                            this.close(this.active, window, cx);
                            cx.stop_propagation();
                        }
                        "o" => {
                            this.pick(cx);
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }
            }))
            .child(tabs)
            .child(div().flex_1().min_h_0().child(content))
    }
}

#[cfg(test)]
mod tests {
    use super::{Workspace, load_recents_from};
    #[test]
    fn recents_fall_back_to_previous_app_then_prefer_new_app() {
        let root = std::env::temp_dir().join(format!(
            "informe-recents-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let new = root.join("Informe");
        let old = root.join("Artifact Preview");
        std::fs::create_dir_all(&new).unwrap();
        std::fs::create_dir_all(&old).unwrap();
        let old_document = root.join("old.md");
        let new_document = root.join("new.md");
        std::fs::write(
            old.join("recent.json"),
            serde_json::to_vec(&vec![&old_document]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            load_recents_from([new.clone(), old.clone()]),
            [old_document]
        );
        std::fs::write(
            new.join("recent.json"),
            serde_json::to_vec(&vec![&new_document]).unwrap(),
        )
        .unwrap();
        assert_eq!(load_recents_from([new, old]), [new_document]);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[gpui::test]
    fn canonical_paths_reuse_tabs_and_closing_preserves_active_document(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_base::init);
        let dir = std::env::temp_dir().join(format!("artifact-tabs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.json");
        let b = dir.join("b.json");
        let alias = dir.join("alias.json");
        for p in [&a, &b] {
            std::fs::write(p, include_bytes!("../examples/dashboard.json")).unwrap();
        }
        let _ = std::fs::remove_file(&alias);
        std::os::unix::fs::symlink(&a, &alias).unwrap();
        let (view, cx) =
            cx.add_window_view(|_, cx| Workspace::new(None, None, Some(dir.join("marker")), cx));
        view.update(cx, |v, cx| {
            v.open(a.clone(), None, cx);
            v.open(b.clone(), None, cx);
            v.open(alias, None, cx);
            assert_eq!(v.documents.len(), 2);
            assert_eq!(v.active, 0);
            v.open(b.clone(), None, cx);
            assert_eq!(v.active, 1);
        });
        cx.update(|window, cx| {
            view.update(cx, |v, cx| {
                v.close(0, window, cx);
                assert_eq!(v.active, 0);
                assert_eq!(v.documents[0].read(cx).path, b.canonicalize().unwrap());
                v.close(0, window, cx);
                assert!(v.documents.is_empty());
            })
        });
        let _ = std::fs::remove_dir_all(dir);
    }
}
