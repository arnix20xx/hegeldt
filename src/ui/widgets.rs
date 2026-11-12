use gpui::{Div, ElementId, Stateful, div, prelude::*, px};

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

/// A square-ish round icon-like button used for the volume controls.
pub fn big_button(id: impl Into<ElementId>, label: &'static str) -> Stateful<Div> {
    button(id, label).size(px(64.)).rounded_full().text_2xl()
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
