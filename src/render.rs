//! Renderer-owned styling: none of these colors or paddings enter the artifact contract.
use crate::{
    artifact::{ArtifactNode, CalloutKind},
    preview::Preview,
};
use gpui::{prelude::*, *};

pub const INK: u32 = 0x202a40;
pub const MUTED: u32 = 0x6c7890;
pub const ACCENT: u32 = 0x5468da;
pub const BORDER: u32 = 0xe1e6ef;
pub const PAPER: u32 = 0xffffff;
pub const BACKGROUND: u32 = 0xf5f7fb;

#[derive(Clone, Copy)]
pub struct TextMode {
    pub all: bool,
    pub components: bool,
}

pub fn render_node(
    node: &ArtifactNode,
    selected: Option<&str>,
    mode: TextMode,
    width: f32,
    viewer: &crate::viewer::ViewerUi,
    changes: &[crate::review::Change],
    cx: &mut Context<Preview>,
) -> AnyElement {
    let id = node.id().to_owned();
    let is_selected = selected == Some(id.as_str());
    // Every artifact node owns one hit target. Stable element IDs are namespaced
    // separately from the viewer chrome. Borders are always allocated.
    let mut element = div()
        .id(SharedString::from(format!("artifact:{id}")))
        .debug_selector(|| id.clone())
        .min_w_0()
        .border_1()
        .rounded_md()
        .border_color(if is_selected {
            rgb(ACCENT).into()
        } else {
            transparent_black()
        })
        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
            if !crate::preview::advanced_tools_enabled()
                || (!this.component_mode && !event.modifiers().alt)
            {
                return;
            }
            cx.stop_propagation();
            if event.modifiers().alt {
                this.component_mode = true;
            }
            this.selected_id = Some(id.clone());
            cx.notify();
        }));

    element = match node {
        ArtifactNode::Column { gap, children, .. } => element
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(gap.unwrap_or(16.)))
            .children(
                children
                    .iter()
                    .map(|child| render_node(child, selected, mode, width, viewer, changes, cx)),
            ),
        ArtifactNode::Row { gap, children, .. } => element
            .w_full()
            .flex()
            .when(width >= 640., |d| d.flex_row())
            .when(width < 640., |d| d.flex_col())
            .items_start()
            .gap(px(gap.unwrap_or(12.)))
            .children(children.iter().map(|child| {
                div().flex_1().w_full().min_w_0().child(render_node(
                    child,
                    selected,
                    mode,
                    if width >= 640. {
                        width / children.len().max(1) as f32
                    } else {
                        width
                    },
                    viewer,
                    changes,
                    cx,
                ))
            })),
        ArtifactNode::Heading { level, text, .. } => element
            .role(Role::Heading)
            .aria_level(*level as usize)
            .aria_label(text.clone())
            .w_full()
            .px_1()
            .text_color(rgb(INK))
            .font_weight(FontWeight::BOLD)
            .text_size(px(match level {
                1 => 30.,
                2 => 22.,
                3 => 18.,
                4 => 18.,
                5 => 16.,
                _ => 14.,
            }))
            .child(selectable(node.id(), "text", text.clone(), mode)),
        ArtifactNode::Markdown { id, .. } => {
            let preview = cx.weak_entity();
            element
                .role(Role::Paragraph)
                .aria_label(node.text_content())
                .w_full()
                .text_size(px(15.))
                .text_color(rgb(INK))
                .when_some(viewer.markdown.get(id), |d, (_, state)| {
                    d.child(
                        gpui_base::text::TextView::new(state)
                            .selectable(!mode.components)
                            .on_link_click(move |url, event, window, cx| {
                                let primary = match event {
                                    gpui::ClickEvent::Mouse(c) => matches!(
                                        c.up.button,
                                        gpui::MouseButton::Left | gpui::MouseButton::Middle
                                    ),
                                    gpui::ClickEvent::Keyboard(_) => true,
                                    gpui::ClickEvent::Touch(c) => !c.long_press,
                                };
                                if !primary {
                                    return;
                                }
                                if let Some(fragment) = url.strip_prefix('#') {
                                    let _ = preview.update(cx, |p, cx| {
                                        if let Some((id, _, _)) =
                                            p.viewer.outline.iter().find(|(_, title, _)| {
                                                crate::viewer::slug(title) == fragment
                                            })
                                            && let Some(anchor) = p.viewer.anchors.get(id)
                                        {
                                            anchor.scroll_to(window, cx);
                                            cx.notify();
                                        }
                                    });
                                } else if url.starts_with("https://")
                                    || url.starts_with("http://")
                                    || url.starts_with("mailto:")
                                {
                                    cx.open_url(url);
                                }
                            })
                            .style(
                                gpui_base::text::TextViewStyle::default()
                                    .with_link(rgb(ACCENT).into())
                                    .with_foreground(rgb(INK).into()),
                            ),
                    )
                })
        }
        ArtifactNode::Text { text, .. } => element
            .w_full()
            .px_1()
            .text_size(px(15.))
            .text_color(rgb(0x3a4658))
            .line_height(gpui::relative(1.6))
            .child(selectable(node.id(), "text", text.clone(), mode)),
        ArtifactNode::Card { children, .. } => element
            .w_full()
            .p_5()
            .bg(rgb(PAPER))
            .rounded_xl()
            .border_color(if is_selected {
                rgb(ACCENT)
            } else {
                rgb(BORDER)
            })
            .flex()
            .flex_col()
            .items_start()
            .gap_4()
            .children(
                children
                    .iter()
                    .map(|child| render_node(child, selected, mode, width, viewer, changes, cx)),
            ),
        ArtifactNode::Metric {
            label,
            value,
            secondary,
            ..
        } => {
            let mut card = element
                .w_full()
                .p_5()
                .rounded_xl()
                .bg(rgb(PAPER))
                .border_color(if is_selected {
                    rgb(ACCENT)
                } else {
                    rgb(BORDER)
                })
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(MUTED))
                        .child(selectable(node.id(), "label", label.clone(), mode)),
                )
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(INK))
                        .child(selectable(node.id(), "value", value.clone(), mode)),
                );
            if let Some(secondary) = secondary {
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(ACCENT))
                        .child(selectable(node.id(), "secondary", secondary.clone(), mode)),
                );
            }
            card
        }
        ArtifactNode::Callout {
            kind, title, body, ..
        } => {
            let (bg, color, symbol) = match kind {
                CalloutKind::Info => (0xeef3ff, 0x4263b8, "i"),
                CalloutKind::Warning => (0xfff5df, 0x966719, "!"),
                CalloutKind::Error => (0xffeeee, 0xb74b4b, "!"),
                CalloutKind::Success => (0xeaf7f0, 0x2b8059, "+"),
            };
            let mut content = div().flex_1().min_w_0().flex().flex_col().gap_2();
            if let Some(title) = title {
                content = content.child(div().font_weight(FontWeight::SEMIBOLD).child(selectable(
                    node.id(),
                    "title",
                    title.clone(),
                    mode,
                )));
            }
            content = content.child(div().text_size(px(14.)).child(selectable(
                node.id(),
                "body",
                body.clone(),
                mode,
            )));
            element
                .w_full()
                .p_5()
                .rounded_lg()
                .bg(rgb(bg))
                .text_color(rgb(color))
                .flex()
                .gap_3()
                .items_start()
                .child(
                    div()
                        .flex_shrink_0()
                        .w_5()
                        .h_5()
                        .rounded_full()
                        .bg(rgb(color))
                        .text_color(rgb(bg))
                        .flex()
                        .items_center()
                        .justify_center()
                        .font_weight(FontWeight::BOLD)
                        .child(symbol),
                )
                .child(content)
        }
        ArtifactNode::Divider { .. } => element
            .w_full()
            .py_2()
            .child(div().h(px(1.)).w_full().bg(rgb(BORDER))),
        ArtifactNode::Button { label, action, .. } => {
            // Keep legacy documents readable without presenting an inert action.
            match action {
                crate::artifact::ArtifactAction::Noop => {}
            }
            element
                .px_5()
                .py_3()
                .bg(rgb(BACKGROUND))
                .text_color(rgb(MUTED))
                .border_color(rgb(BORDER))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(14.))
                .child(selectable(
                    node.id(),
                    "label",
                    format!("{label} · unavailable"),
                    mode,
                ))
        }
        ArtifactNode::Chart {
            title,
            kind,
            unit,
            data,
            ..
        } => element
            .role(Role::Group)
            .aria_label(title.clone())
            .w_full()
            .p_5()
            .bg(rgb(PAPER))
            .rounded_xl()
            .border_color(if is_selected {
                rgb(ACCENT)
            } else {
                rgb(BORDER)
            })
            .child(crate::data_visuals::render_chart(
                title,
                *kind,
                unit.as_deref(),
                data,
                node.id(),
                mode,
            )),
        ArtifactNode::Table {
            title,
            columns,
            rows,
            ..
        } => element
            .role(Role::Table)
            .aria_label(title.clone())
            .aria_row_count(rows.len() + 1)
            .aria_column_count(columns.len())
            .w_full()
            .p_5()
            .bg(rgb(PAPER))
            .rounded_xl()
            .border_color(if is_selected {
                rgb(ACCENT)
            } else {
                rgb(BORDER)
            })
            .child(crate::data_visuals::render_table(
                title,
                columns,
                rows,
                node.id(),
                mode,
                viewer,
                cx,
            )),
    };
    if !is_selected && let Some(change) = changes.iter().find(|c| c.node_id == node.id()) {
        element = element.border_color(rgb(match change.kind {
            crate::review::ChangeKind::Added => 0x2b8059,
            _ => 0xd3a13c,
        }));
    }
    let anchor = viewer.anchors.get(node.id()).cloned();
    let found = viewer.find
        && viewer
            .matches
            .get(viewer.current)
            .is_some_and(|(id, _)| id == node.id());
    element
        .anchor_scroll(anchor)
        .when(found, |d| d.border_color(rgb(0xd3a13c)).bg(rgb(0xfffaf0)))
        .into_any_element()
}

/// Independent text runs participate in GPUI Kit's window-wide selection layer.
pub fn selectable(
    id: &str,
    field: &str,
    text: impl Into<SharedString>,
    mode: TextMode,
) -> AnyElement {
    let text = text.into();
    if mode.components {
        return div().child(text).into_any_element();
    }
    div()
        .id(SharedString::from(format!("a11y:{id}:{field}")))
        .role(Role::Label)
        .aria_label(text.clone())
        .when(mode.all, |d| d.bg(rgb(0xd9e4ff)))
        .child(gpui_base::SelectableText::new(
            SharedString::from(format!("text:{id}:{field}")),
            text,
        ))
        .into_any_element()
}
