use glimpse_core::{GitChange, Repository};
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

pub struct ChangeSelected(pub GitChange);

pub struct Changes {
    pub repository: Repository,
    selected: Option<usize>,
    scroll: UniformListScrollHandle,
}

impl EventEmitter<ChangeSelected> for Changes {}

impl Changes {
    pub fn new(repository: Repository) -> Self {
        Self {
            repository,
            selected: None,
            scroll: UniformListScrollHandle::new(),
        }
    }
}

impl Render for Changes {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Changes across the Git repository"),
            )
            .when(self.repository.changes.is_empty(), |view| {
                view.child(div().p_4().text_sm().child("Working tree clean."))
            })
            .child(
                uniform_list(
                    "changes",
                    self.repository.changes.len(),
                    cx.processor(|view, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                let change = &view.repository.changes[index];
                                let color = match change.status {
                                    'A' | '?' => cx.theme().green,
                                    'D' | 'U' => cx.theme().red,
                                    _ => cx.theme().foreground,
                                };
                                div()
                                    .h(px(56.))
                                    .px_2()
                                    .when(view.selected == Some(index), |row| {
                                        row.bg(cx.theme().accent)
                                    })
                                    .child(
                                        Button::new(("change", index))
                                            .ghost()
                                            .accessibility_label(format!(
                                                "{} · {} · {}",
                                                change.path.display(),
                                                change.scope.label(),
                                                change.status
                                            ))
                                            .child(div().w_full().text_left().truncate().child(
                                                format!(
                                                    "{}  {}",
                                                    change.status,
                                                    change.path.display()
                                                ),
                                            ))
                                            .text_color(color)
                                            .w_full()
                                            .h(px(30.))
                                            .justify_start()
                                            .on_click(cx.listener(move |view, _, _, cx| {
                                                view.selected = Some(index);
                                                cx.emit(ChangeSelected(
                                                    view.repository.changes[index].clone(),
                                                ));
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .pl_3()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(change.scope.label()),
                                    )
                            })
                            .collect()
                    }),
                )
                .flex_1()
                .min_h_0()
                .track_scroll(&self.scroll),
            )
    }
}
