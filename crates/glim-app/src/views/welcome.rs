use gpui_kit::{
    component::{ActiveTheme, v_flex},
    *,
};

pub fn welcome(cx: &App) -> impl IntoElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
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
                .child("AI writes. You see."),
        )
        .child(
            div()
                .mt_4()
                .text_sm()
                .child("Open a folder to explore code, changes, and Markdown."),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("⌘⇧O  Open folder     ·     ⌘O  Open file"),
        )
}
