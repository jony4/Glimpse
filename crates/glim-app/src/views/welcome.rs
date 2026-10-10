use crate::app::actions::OpenFolder;
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

pub fn welcome(show_open_folder: bool, cx: &App) -> impl IntoElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .px_6()
        .text_center()
        .child(img("branding/glim.png").w(px(88.)).h(px(88.)).mb_3())
        .child(
            div()
                .text_3xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Glim"),
        )
        .child(
            div()
                .text_color(cx.theme().muted_foreground)
                .child("The all-purpose viewer for the AI era."),
        )
        .when(show_open_folder, |view| {
            view.child(
                div().mt_4().child(
                    Button::new("welcome-open-folder")
                        .primary()
                        .label("Open Folder")
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(OpenFolder), cx);
                        }),
                ),
            )
        })
}
