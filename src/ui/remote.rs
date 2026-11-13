use gpui::{Context, Corner, MouseButton, Window, anchored, deferred, div, point, prelude::*, px};

use super::{theme, widgets};
use crate::app::{Dir, RemoteApp};

impl RemoteApp {
    pub fn render_remote(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sources = self.sources();
        let current = self
            .input
            .and_then(|id| sources.iter().find(|s| s.id == id))
            .map(|s| s.display_name().to_string())
            .unwrap_or_default();
        let controls_on = self.power && self.connected;

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
                            .child(if self.power {
                                current.clone()
                            } else {
                                String::new()
                            }),
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

        let mut root = div().flex().flex_col().size_full().child(display);

        // --- middle: input dropdown + volume ---
        let middle = if controls_on {
            let shown = if current.is_empty() {
                "Select input".to_string()
            } else {
                current
            };
            let mut dropdown = div().relative().flex_1().child(
                widgets::button("input-dropdown", div().flex_1().child(shown))
                    .h(px(44.))
                    .justify_between()
                    .gap_2()
                    .tooltip(widgets::tip("Select input (1-9)"))
                    .child(widgets::icon("icons/chevron-down.svg", 20., theme::muted()))
                    .on_click(cx.listener(|t, _, _, cx| {
                        t.dropdown_open = !t.dropdown_open;
                        cx.notify();
                    })),
            );
            if self.dropdown_open {
                let mut list = div()
                    .id("source-list")
                    .flex()
                    .flex_col()
                    .w(px(190.))
                    .max_h(px(180.))
                    .overflow_y_scroll()
                    .p_1()
                    .rounded_md()
                    .bg(theme::surface())
                    .border_1()
                    .border_color(theme::surface_active())
                    .shadow_lg()
                    .on_mouse_down_out(cx.listener(|t, _, _, cx| {
                        t.dropdown_open = false;
                        cx.notify();
                    }));
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
                            .on_click(cx.listener(move |t, _, _, cx| {
                                t.select_input(id);
                                t.dropdown_open = false;
                                cx.notify();
                            }))
                            .child(s.display_name().to_string())
                            .child(div().text_xs().text_color(theme::muted()).child(if n < 9 {
                                (n + 1).to_string()
                            } else {
                                String::new()
                            })),
                    );
                }
                dropdown = dropdown.child(deferred(
                    anchored()
                        .anchor(Corner::TopLeft)
                        .offset(point(px(0.), px(48.)))
                        .snap_to_window_with_margin(px(8.))
                        .child(list),
                ));
            }

            let vol_btn = |id: &'static str,
                           icon: &'static str,
                           tip: &'static str,
                           dir: Dir,
                           cx: &mut Context<Self>| {
                widgets::icon_button(id, icon, theme::text(), tip)
                    .w(px(56.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |t, _, _, cx| t.start_hold(dir, cx)),
                    )
                    .on_mouse_up(MouseButton::Left, cx.listener(|t, _, _, _| t.stop_hold()))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|t, _, _, _| t.stop_hold()))
            };
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .pt_4()
                .child(dropdown)
                .child(vol_btn(
                    "vol-down",
                    "icons/volume-down.svg",
                    "Volume down (↓)",
                    Dir::Down,
                    cx,
                ))
                .child(vol_btn(
                    "vol-up",
                    "icons/volume-up.svg",
                    "Volume up (↑)",
                    Dir::Up,
                    cx,
                ))
        } else {
            div()
        };
        root = root.child(middle);

        if let Some(msg) = self.message.clone() {
            root = root.child(
                div()
                    .px_4()
                    .pt_3()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(msg),
            );
        }
        root = root.child(div().flex_1());

        // --- bottom bar: power, mute, settings ---
        let (mute_icon, mute_color, mute_tip) = if self.mute {
            ("icons/volume-off.svg", theme::error(), "Unmute (M)")
        } else {
            ("icons/volume-on.svg", theme::text(), "Mute (M)")
        };
        let power_color = if self.power {
            theme::accent()
        } else {
            theme::text()
        };
        root.child(
            div()
                .flex()
                .gap_2()
                .p_3()
                .child(
                    widgets::icon_button(
                        "power",
                        "icons/power.svg",
                        power_color,
                        if self.power {
                            "Power off (P)"
                        } else {
                            "Power on (P)"
                        },
                    )
                    .flex_1()
                    .on_click(cx.listener(|t, _, _, _| t.toggle_power())),
                )
                .child(
                    widgets::icon_button("mute", mute_icon, mute_color, mute_tip)
                        .flex_1()
                        .on_click(cx.listener(|t, _, _, _| t.toggle_mute())),
                )
                .child(
                    widgets::icon_button(
                        "settings",
                        "icons/settings.svg",
                        theme::text(),
                        "Settings (⌘,)",
                    )
                    .flex_1()
                    .on_click(cx.listener(|t, _, w, cx| t.open_settings(w, cx))),
                ),
        )
    }
}
