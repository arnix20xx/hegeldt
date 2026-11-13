use gpui::{
    AnyView, App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, prelude::*, px, svg,
};

use gpui::Context;

use super::theme;

/// A plain rounded button. The caller attaches click / mouse handlers.
pub fn button(id: impl Into<ElementId>, label: impl IntoElement) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme::surface())
        .text_color(theme::text())
        .cursor_pointer()
        .hover(|s| s.bg(theme::surface_hover()))
        .active(|s| s.bg(theme::surface_active()))
        .child(label)
}

/// A small SVG icon from `assets/icons/` (see `assets.rs`), tinted with `color`.
pub fn icon(path: &'static str, size: f32, color: Rgba) -> gpui::Svg {
    svg()
        .path(path)
        .size(px(size))
        .flex_none()
        .text_color(color)
}

struct TextTooltip(SharedString);

impl Render for TextTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(theme::surface_active())
            .text_color(theme::text())
            .text_xs()
            .child(self.0.clone())
    }
}

/// Builder for `.tooltip(...)`.
pub fn tip(text: &'static str) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| TextTooltip(text.into())).into()
}

/// Round-cornered button showing only an icon.
pub fn icon_button(
    id: impl Into<ElementId>,
    path: &'static str,
    color: Rgba,
    tooltip: &'static str,
) -> Stateful<Div> {
    button(id, icon(path, 24., color))
        .h(px(44.))
        .tooltip(tip(tooltip))
}

/// On/off switch drawn with divs.
pub fn toggle(id: impl Into<ElementId>, on: bool) -> Stateful<Div> {
    let knob = div()
        .size(px(16.))
        .rounded_full()
        .bg(theme::text())
        .when(on, |d| d.ml(px(16.)));
    div()
        .id(id)
        .w(px(40.))
        .h(px(20.))
        .p(px(2.))
        .flex()
        .items_center()
        .rounded_full()
        .cursor_pointer()
        .bg(if on {
            theme::ok()
        } else {
            theme::surface_active()
        })
        .child(knob)
}

pub fn section_title(text: &'static str) -> Div {
    div()
        .text_xs()
        .text_color(theme::muted())
        .mt_3()
        .mb_1()
        .child(text)
}
