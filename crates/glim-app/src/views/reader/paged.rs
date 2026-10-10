use glim_services::paged::{FULL_BYTES, TextPage, read_page};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};
use std::ops::Range;

/// Virtual rows contain slices into one buffer, not one allocation per line.
/// Paging replaces the buffer, so browsing an unbounded file stays bounded.
pub(super) struct PagedReader {
    page: TextPage,
    rows: Vec<Range<usize>>,
    widest_row: Option<usize>,
    previous: Vec<u64>,
    task: Option<Task<()>>,
    error: Option<String>,
    scroll: UniformListScrollHandle,
}
impl PagedReader {
    pub fn new(page: TextPage) -> Self {
        let rows = rows(&page.text);
        let widest_row = widest_row(&page.text, &rows);
        Self {
            widest_row,
            page,
            rows,
            previous: Vec::new(),
            task: None,
            error: None,
            scroll: UniformListScrollHandle::new(),
        }
    }
    fn load(
        &mut self,
        start: u64,
        all: bool,
        back: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.task.is_some() {
            return;
        }
        let path = self.page.path.clone();
        let stamp = (self.page.total, self.page.modified);
        let response = all.then(|| window.prompt(PromptLevel::Warning, "Load the entire file?", Some("Full view can use substantially more memory. Files up to 64 MiB can be loaded as read-only virtual text; syntax parsing, editing and minimap are disabled. Larger files remain paged."), &["Cancel", "Load All"], cx));
        self.task = Some(cx.spawn_in(window, async move |view, cx| {
            if let Some(response) = response
                && response.await != Ok(1)
            {
                let _ = view.update(cx, |v, cx| {
                    v.task = None;
                    cx.notify();
                });
                return;
            }
            let result = cx
                .background_executor()
                .spawn(async move {
                    let page = read_page(&path, start, all)?;
                    anyhow::ensure!(
                        (page.total, page.modified) == stamp,
                        "File changed on disk. Reopen or refresh before paging."
                    );
                    let rows = rows(&page.text);
                    let widest = widest_row(&page.text, &rows);
                    Ok::<_, anyhow::Error>((page, rows, widest))
                })
                .await;
            let _ = view.update(cx, |v, cx| {
                v.task = None;
                match result {
                    Ok((page, rows, widest)) => {
                        if all {
                            v.previous.clear();
                        } else if back {
                            v.previous.pop();
                        } else {
                            v.previous.push(v.page.start);
                        }
                        v.widest_row = widest;
                        v.page = page;
                        v.rows = rows;
                        v.error = None;
                        v.scroll = UniformListScrollHandle::new();
                    }
                    Err(e) => v.error = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
fn rows(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = 0;
    // Cap individual layout runs, including minified multi-megabyte JSON lines.
    for (offset, ch) in text.char_indices() {
        if ch == '\n' || offset - start >= 1024 {
            result.push(start..offset);
            start = if ch == '\n' { offset + 1 } else { offset };
        }
    }
    if start < text.len() {
        result.push(start..text.len());
    }
    result
}
// Select once per loaded buffer, outside render. The list measures this row
// with the active font and padding, and clamps its extent to the viewport.
fn widest_row(text: &str, rows: &[Range<usize>]) -> Option<usize> {
    rows.iter()
        .enumerate()
        .max_by_key(|(_, range)| text[(*range).clone()].chars().count())
        .map(|(index, _)| index)
}
impl Render for PagedReader {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let previous = self.previous.last().copied();
        let next = self.page.end;
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .px_3()
                    .py_1()
                    .gap_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(div().flex_1().min_w_0().truncate().child(format!(
                        "Read-only · bytes {}–{} / {} · long lines split for display",
                        self.page.start, self.page.end, self.page.total
                    )))
                    .child(
                        Button::new("page-back")
                            .ghost()
                            .label("Previous")
                            .disabled(busy || previous.is_none())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                if let Some(offset) = previous {
                                    v.load(offset, false, true, w, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("page-next")
                            .ghost()
                            .label("Next page")
                            .disabled(busy || next >= self.page.total)
                            .on_click(
                                cx.listener(move |v, _, w, cx| v.load(next, false, false, w, cx)),
                            ),
                    )
                    .child(
                        Button::new("page-all")
                            .ghost()
                            .label("Load all…")
                            .tooltip("Full view: up to 64 MiB and one million line breaks; larger files stay paged")
                            .disabled(
                                busy || self.page.total > FULL_BYTES
                                    || (self.page.start == 0 && self.page.end == self.page.total),
                            )
                            .on_click(cx.listener(|v, _, w, cx| v.load(0, true, false, w, cx))),
                    ),
            )
            .children(self.error.iter().map(|e| {
                div()
                    .px_3()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(e.clone())
            }))
            .child(
                uniform_list(
                    "large-text",
                    self.rows.len(),
                    cx.processor(|v, range: Range<usize>, _, cx| {
                        range.map(|i| {
                            div()
                                .h(px(22.))
                                .px_3()
                                .text_sm()
                                .font_family("Menlo")
                                .whitespace_nowrap()
                                .text_color(cx.theme().foreground)
                                .child(v.page.text[v.rows[i].clone()].to_owned())
                        }).collect()
                    }),
                )
                .with_width_from_item(self.widest_row)
                .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0()
                .w_full(),
            )
    }
}
