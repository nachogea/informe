//! Small data displays using GPUI layout primitives; no external chart runtime.
use crate::{
    artifact::{ChartDatum, ChartKind},
    render::{ACCENT, BACKGROUND, BORDER, INK, MUTED, TextMode, selectable},
};
use gpui::{Div, FontWeight, div, prelude::*, px, relative, rgb};

pub fn render_chart(
    title: &str,
    kind: ChartKind,
    unit: Option<&str>,
    data: &[ChartDatum],
    id: &str,
    mode: TextMode,
) -> Div {
    match kind {
        ChartKind::Bar => {}
    }
    let maximum = data.iter().map(|d| d.value).fold(0., f64::max);
    let total: f64 = data.iter().map(|d| d.value).sum();
    let mut chart = div().w_full().flex().flex_col().gap_3().child(
        div()
            .mb_2()
            .text_size(px(17.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(selectable(id, "title", title.to_owned(), mode)),
    );
    for (index, datum) in data.iter().enumerate() {
        let ratio = if maximum > 0. {
            (datum.value / maximum) as f32
        } else {
            0.
        };
        let value = format_value(datum.value, unit);
        let tip = format!(
            "{} · {} · {:.1}% of total",
            datum.label,
            value,
            if total > 0. {
                datum.value / total * 100.
            } else {
                0.
            }
        );
        chart = chart.child(
            div()
                .id(gpui::SharedString::from(format!("chart:{id}:{index}")))
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .rounded_md()
                .hover(|d| d.bg(rgb(BACKGROUND)))
                .tooltip(move |_, cx| cx.new(|_| ChartTip(tip.clone())).into())
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_4()
                        .text_size(px(12.))
                        .child(selectable(
                            id,
                            &format!("label:{index}"),
                            datum.label.clone(),
                            mode,
                        ))
                        .child(div().font_weight(FontWeight::MEDIUM).child(selectable(
                            id,
                            &format!("value:{index}"),
                            value,
                            mode,
                        ))),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(8.))
                        .rounded_md()
                        .bg(rgb(BACKGROUND))
                        .child(
                            div()
                                .w(relative(ratio))
                                .h_full()
                                .rounded_md()
                                .bg(rgb(ACCENT)),
                        ),
                ),
        );
    }
    chart.child(
        div()
            .mt_1()
            .flex()
            .justify_between()
            .text_size(px(10.))
            .text_color(rgb(MUTED))
            .child("0")
            .child(format_value(maximum, unit)),
    )
}
struct ChartTip(String);
impl gpui::Render for ChartTip {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .px_3()
            .py_2()
            .bg(rgb(INK))
            .text_color(rgb(0xffffff))
            .text_size(px(12.))
            .rounded_md()
            .shadow_md()
            .child(self.0.clone())
    }
}

fn format_value(value: f64, unit: Option<&str>) -> String {
    match unit {
        Some("$" | "USD") => {
            let digits = format!("{value:.0}");
            let grouped: String = digits
                .chars()
                .enumerate()
                .flat_map(|(i, c)| {
                    let comma = i > 0 && (digits.len() - i).is_multiple_of(3);
                    comma.then_some(',').into_iter().chain(std::iter::once(c))
                })
                .collect();
            format!("${grouped}")
        }
        Some("%") => format!("{value}%"),
        Some(unit) => format!("{value} {unit}"),
        None => value.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_table(
    title: &str,
    columns: &[String],
    rows: &[Vec<String>],
    id: &str,
    mode: TextMode,
    viewer: &crate::viewer::ViewerUi,
    cx: &mut gpui::Context<crate::preview::Preview>,
) -> Div {
    let sort = viewer.table_sort.get(id).copied();
    let mut header = div()
        .w_full()
        .flex()
        .bg(rgb(BACKGROUND))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(MUTED));
    for (column, label) in columns.iter().enumerate() {
        let id = id.to_owned();
        let arrow = match sort {
            Some((c, true)) if c == column => "↓",
            Some((c, false)) if c == column => "↑",
            _ => "↕",
        };
        header = header.child(
            div()
                .flex_1()
                .min_w_0()
                .px_3()
                .py_3()
                .flex()
                .items_start()
                .justify_between()
                .gap_1()
                .child(selectable(
                    &id,
                    &format!("header:{column}"),
                    label.clone(),
                    mode,
                ))
                .child(
                    div()
                        .id(gpui::SharedString::from(format!("sort:{id}:{column}")))
                        .px_1()
                        .cursor_pointer()
                        .text_color(rgb(ACCENT))
                        .child(arrow)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            gpui_base::TextSelection::clear(window, cx);
                            let reverse = this
                                .viewer
                                .table_sort
                                .get(&id)
                                .is_some_and(|(c, desc)| *c == column && !desc);
                            this.viewer.table_sort.insert(id.clone(), (column, reverse));
                            cx.stop_propagation();
                            cx.notify();
                        })),
                ),
        );
    }
    let mut grid = div()
        .w_full()
        .min_w(px(columns.len() as f32 * 130.))
        .flex()
        .flex_col()
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_md()
        .overflow_hidden()
        .child(header);
    let ordered = sorted_rows(rows, sort);
    for (position, index) in ordered.into_iter().enumerate() {
        grid = grid.child(
            table_row(&rows[index], id, &index.to_string(), mode)
                .border_t_1()
                .border_color(rgb(BORDER))
                .bg(rgb(if position % 2 == 0 {
                    0xffffff
                } else {
                    0xfafbfc
                })),
        );
    }
    if rows.is_empty() {
        grid = grid.child(
            div()
                .p_4()
                .text_size(px(13.))
                .text_color(rgb(MUTED))
                .child("No rows"),
        );
    }
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_size(px(17.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(selectable(id, "title", title.to_owned(), mode)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            table_action(format!("copy-table:{id}"), "Copy TSV").on_click(
                                cx.listener({
                                    let id = id.to_owned();
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.copy_table(&id, cx);
                                    }
                                }),
                            ),
                        )
                        .child(
                            table_action(format!("save-table:{id}"), "Save CSV…").on_click(
                                cx.listener({
                                    let id = id.to_owned();
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.save_table(&id, cx);
                                    }
                                }),
                            ),
                        ),
                ),
        )
        .child(
            div()
                .id(gpui::SharedString::from(format!("table-scroll:{id}")))
                .w_full()
                .overflow_x_scroll()
                .child(grid),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(MUTED))
                .child(format!("{} rows", rows.len())),
        )
}
fn table_action(id: String, label: &str) -> gpui_base::Button {
    gpui_base::Button::new(gpui::SharedString::from(id))
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
fn sorted_rows(rows: &[Vec<String>], sort: Option<(usize, bool)>) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..rows.len()).collect();
    if let Some((col, desc)) = sort {
        fn numeric(s: &str) -> Option<f64> {
            s.trim()
                .trim_start_matches(['$', '£', '€'])
                .trim_end_matches('%')
                .replace(',', "")
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
        }
        let numeric_column = rows
            .iter()
            .any(|row| row.get(col).is_some_and(|cell| !cell.trim().is_empty()))
            && rows.iter().all(|row| {
                row.get(col)
                    .is_none_or(|cell| cell.trim().is_empty() || numeric(cell).is_some())
            });
        indices.sort_by(|a, b| {
            let a = rows[*a].get(col).map(String::as_str).unwrap_or("");
            let b = rows[*b].get(col).map(String::as_str).unwrap_or("");
            let order = match (a.trim().is_empty(), b.trim().is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) if numeric_column => {
                    numeric(a).unwrap().total_cmp(&numeric(b).unwrap())
                }
                (false, false) => a.to_lowercase().cmp(&b.to_lowercase()),
            };
            if desc && !a.trim().is_empty() && !b.trim().is_empty() {
                order.reverse()
            } else {
                order
            }
        });
    }
    indices
}

fn table_row(cells: &[String], id: &str, row: &str, mode: TextMode) -> Div {
    div()
        .w_full()
        .flex()
        .text_size(px(13.))
        .children(cells.iter().enumerate().map(|(col, cell)| {
            div().flex_1().min_w_0().px_4().py_3().child(selectable(
                id,
                &format!("{row}:{col}"),
                cell.clone(),
                mode,
            ))
        }))
}

#[cfg(test)]
mod tests {
    use super::sorted_rows;
    #[test]
    fn numeric_sort_is_stable_and_does_not_change_source_rows() {
        let rows = vec![
            vec!["$1,000".into()],
            vec!["$99".into()],
            vec!["$99".into()],
        ];
        assert_eq!(sorted_rows(&rows, Some((0, false))), [1, 2, 0]);
        assert_eq!(sorted_rows(&rows, Some((0, true))), [0, 1, 2]);
        assert_eq!(sorted_rows(&rows, None), [0, 1, 2]);
        assert_eq!(rows[0][0], "$1,000");
    }

    #[test]
    fn mixed_and_blank_cells_have_consistent_order() {
        for values in [
            ["2", "10", "1a", ""],
            ["1a", "", "2", "10"],
            ["10", "1a", "", "2"],
        ] {
            let rows: Vec<Vec<String>> = values.iter().map(|s| vec![s.to_string()]).collect();
            let ordered = |descending| {
                sorted_rows(&rows, Some((0, descending)))
                    .into_iter()
                    .map(|i| rows[i][0].as_str())
                    .collect::<Vec<_>>()
            };
            assert_eq!(ordered(false), ["10", "1a", "2", ""]);
            assert_eq!(ordered(true), ["2", "1a", "10", ""]);
        }
    }
}
