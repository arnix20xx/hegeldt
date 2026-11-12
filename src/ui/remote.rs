use gpui::{Context, MouseButton, div, prelude::*, px};

use super::{theme, widgets};
use crate::app::{Dir, RemoteApp};

impl RemoteApp {
    pub fn render_remote(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let sources = self.sources();
        let current = self
            .input
            .and_then(|id| sources.iter().find(|s| s.id == id))
            .map(|s| s.display_name().to_string())
            .unwrap_or_default();

        // --- display ---
        let volume_text = if self.power {
            self.volume.to_string()
        } else {
            String::new()
        };
        let volume_alpha = if self.mute && !self.blink_on {
            0.0
        } else {
            1.0
        };
        let display = div()
            .flex()
            .flex_col()
            .w_full()
            .bg(theme::display_bg())
            .px_4()
            .py_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(32.))
                            .text_color(theme::display_text())
                            .child(if self.power { current } else { String::new() }),
                    )
                    .child(
                        div()
                            .text_size(px(50.))
                            .text_color(theme::display_text())
                            .opacity(volume_alpha)
                            .child(volume_text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(6.)).bg(if self.connected {
                        theme::ok()
                    } else {
                        theme::error()
                    }))
                    .child(div().text_xs().text_color(theme::display_text()).child(
                        if self.power {
                            self.config.model.label().to_string()
                        } else {
                            String::new()
                        },
                    )),
            );

        // --- header ---
        let header = div()
            .flex()
            .justify_between()
            .items_center()
            .px_4()
            .py_3()
            .child(
                div()
                    .text_lg()
                    .text_color(theme::display_text())
                    .child("HEGEL"),
            )
            .child(
                widgets::button("settings", "Settings")
                    .on_click(cx.listener(|t, _, _, cx| t.open_settings(cx))),
            );

        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .child(display);

        if let Some(msg) = self.message.clone() {
            root = root.child(div().p_4().text_sm().text_color(theme::muted()).child(msg));
        }

        // --- controls (only when powered on) ---
        if self.power && self.connected {
            let mut list = div()
                .id("source-list")
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .pr_2();
            for (n, s) in sources.iter().filter(|s| s.enabled).enumerate() {
                let id = s.id;
                let selected = self.input == Some(id);
                list = list.child(
                    div()
                        .id(("src", id as usize))
                        .flex()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(if selected {
                            theme::surface_active()
                        } else {
                            theme::surface()
                        })
                        .hover(|s| s.bg(theme::surface_hover()))
                        .on_click(cx.listener(move |t, _, _, _| t.select_input(id)))
                        .child(s.display_name().to_string())
                        .child(div().text_xs().text_color(theme::muted()).child(if n < 9 {
                            (n + 1).to_string()
                        } else {
                            String::new()
                        })),
                );
            }

            let vol_btn =
                |id: &'static str, label: &'static str, dir: Dir, cx: &mut Context<Self>| {
                    widgets::big_button(id, label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |t, _, _, cx| t.start_hold(dir, cx)),
                        )
                        .on_mouse_up(MouseButton::Left, cx.listener(|t, _, _, _| t.stop_hold()))
                        .on_mouse_up_out(MouseButton::Left, cx.listener(|t, _, _, _| t.stop_hold()))
                };
            let up = vol_btn("vol-up", "+", Dir::Up, cx);
            let down = vol_btn("vol-down", "−", Dir::Down, cx);

            root = root.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .py_4()
                    .gap_4()
                    .child(list)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_4()
                            .child(div().text_color(theme::muted()).child("Volume"))
                            .child(up)
                            .child(down),
                    ),
            );
        } else {
            root = root.child(div().flex_1());
        }

        // --- bottom bar ---
        root.child(
            div()
                .flex()
                .gap_2()
                .p_3()
                .child(
                    widgets::button("power", "Power")
                        .flex_1()
                        .on_click(cx.listener(|t, _, _, _| t.toggle_power())),
                )
                .child(
                    widgets::button("mute", if self.mute { "Unmute" } else { "Mute" })
                        .flex_1()
                        .on_click(cx.listener(|t, _, _, _| t.toggle_mute())),
                ),
        )
    }
}
