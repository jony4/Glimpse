use glim_services::json::JsonNode;
use gpui_kit::{
    component::{ActiveTheme, Icon, v_flex},
    prelude::FluentBuilder,
    *,
};
use std::collections::HashSet;

pub(super) struct JsonTree {
    nodes: Vec<JsonNode>,
    expanded: HashSet<usize>,
    rows: Vec<(usize, usize)>,
    task: Option<Task<()>>,
    error: Option<String>,
    scroll: UniformListScrollHandle,
}
impl JsonTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            expanded: HashSet::new(),
            rows: Vec::new(),
            task: None,
            error: None,
            scroll: UniformListScrollHandle::new(),
        }
    }
    pub fn load(&mut self, text: String, cx: &mut Context<Self>) {
        self.nodes.clear();
        self.rows.clear();
        self.expanded.clear();
        self.error = None;
        self.scroll = UniformListScrollHandle::new();
        let work = cx
            .background_executor()
            .spawn(async move { glim_services::json::parse(&text) });
        self.task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |v, cx| {
                v.task = None;
                match result {
                    Ok(nodes) => { v.nodes = nodes; v.expanded.insert(0); v.rebuild(); }
                    Err(e) => v.error = Some(format!("Cannot show JSON tree: {e}. Switch to Source to view or edit. Tree view accepts standard JSON (no comments or JSONL).")),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn rebuild(&mut self) {
        self.rows.clear();
        if self.nodes.is_empty() {
            return;
        }
        let mut pending = vec![(0, 0)];
        while let Some((index, depth)) = pending.pop() {
            self.rows.push((index, depth));
            if self.expanded.contains(&index) {
                pending.extend(
                    self.nodes[index]
                        .children
                        .iter()
                        .rev()
                        .map(|i| (*i, depth + 1)),
                );
            }
        }
    }
}
impl Render for JsonTree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .when(self.task.is_some(), |v| {
                v.child(div().p_4().child("Loading JSON tree…"))
            })
            .when_some(self.error.clone(), |v, error| {
                v.child(
                    div()
                        .p_4()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(error),
                )
            })
            .child(
                uniform_list(
                    "json-tree",
                    self.rows.len(),
                    cx.processor(|v, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|row| {
                                let (index, depth) = v.rows[row];
                                let node = &v.nodes[index];
                                let expandable = !node.children.is_empty();
                                let expanded = v.expanded.contains(&index);
                                let color = match node.kind {
                                    "string" => cx.theme().green,
                                    "number" => cx.theme().blue,
                                    "boolean" => cx.theme().magenta,
                                    _ => cx.theme().muted_foreground,
                                };
                                div()
                                    .id(("json-node", index))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .h(px(28.))
                                    .pl(px(12. + depth.min(20) as f32 * 16.))
                                    .pr_3()
                                    .text_sm()
                                    .when(expandable, |row| row.cursor_pointer())
                                    .child(div().w(px(14.)).flex_shrink_0().when(expandable, |v| {
                                        v.child(Icon::new(if expanded {
                                            gpui_kit::assets::IconName::ChevronDown
                                        } else {
                                            gpui_kit::assets::IconName::ChevronRight
                                        }))
                                    }))
                                    .child(
                                        div()
                                            .max_w(relative(0.45))
                                            .truncate()
                                            .text_color(cx.theme().blue)
                                            .child(node.key.clone()),
                                    )
                                    .child(div().text_color(cx.theme().muted_foreground).child(":"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .text_color(color)
                                            .child(node.value.clone()),
                                    )
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        if expandable {
                                            if !v.expanded.remove(&index) {
                                                v.expanded.insert(index);
                                            }
                                            v.rebuild();
                                            cx.notify();
                                        }
                                    }))
                            })
                            .collect()
                    }),
                )
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0()
                .w_full(),
            )
    }
}
