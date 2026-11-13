use gpui::{Context, div, prelude::*, px};
use hegeldt::model::AmpModel;

use super::{theme, widgets};
use crate::app::{Field, RemoteApp, SearchState};

impl RemoteApp {
    fn text_field(
        &self,
        id: impl Into<gpui::ElementId>,
        field: Field,
        shown: String,
        placeholder: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let editing = self.editing == Some(field);
        let text = if editing {
            format!("{}▏", self.buffer)
        } else if shown.is_empty() {
            placeholder.to_string()
        } else {
            shown.clone()
        };
        div()
            .id(id)
            .px_3()
            .py_2()
            .rounded_md()
            .cursor_text()
            .bg(theme::surface())
            .border_1()
            .border_color(if editing {
                theme::accent()
            } else {
                theme::surface()
            })
            .text_color(if !editing && shown.is_empty() {
                theme::muted()
            } else {
                theme::text()
            })
            .child(text)
            .on_click(cx.listener(move |t, _, _, cx| {
                t.begin_edit(field);
                cx.notify();
            }))
    }

    /// The "search for amplifiers" button and its result list.
    fn render_search(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let searching = self.search == SearchState::Searching;
        let mut col = div().mt_2().flex().flex_col().gap_1().child(
            widgets::button(
                "search",
                if searching {
                    "Searching…"
                } else {
                    "Search for amplifiers"
                },
            )
            .text_color(if searching {
                theme::muted()
            } else {
                theme::text()
            })
            .on_click(cx.listener(|t, _, _, cx| t.start_search(cx))),
        );
        match self.search.clone() {
            SearchState::Idle | SearchState::Searching => {}
            SearchState::Failed(e) => {
                col = col.child(div().text_sm().text_color(theme::error()).child(e));
            }
            SearchState::Found(devs) if devs.is_empty() => {
                col = col.child(
                    div()
                        .text_sm()
                        .text_color(theme::muted())
                        .child("No Hegel devices found"),
                );
            }
            SearchState::Found(devs) => {
                for (i, d) in devs.into_iter().enumerate() {
                    let picked = self.picked.as_deref() == Some(d.host.as_str());
                    let title = if d.friendly_name.is_empty() {
                        d.model.clone()
                    } else {
                        d.friendly_name.clone()
                    };
                    let dev = d.clone();
                    col = col.child(
                        div()
                            .id(("dev", i))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(theme::surface())
                            .border_1()
                            .border_color(if picked {
                                theme::ok()
                            } else {
                                theme::surface()
                            })
                            .hover(|s| s.bg(theme::surface_hover()))
                            .on_click(cx.listener(move |t, _, _, cx| {
                                t.pick_device(&dev);
                                cx.notify();
                            }))
                            .child(
                                div().flex().flex_col().child(div().child(title)).child(
                                    div()
                                        .text_xs()
                                        .text_color(theme::muted())
                                        .child(d.model.clone()),
                                ),
                            )
                            .child(div().text_sm().child(if picked {
                                format!("✓ {}", d.host)
                            } else {
                                d.host.clone()
                            })),
                    );
                }
                if self.picked.is_some() {
                    col = col.child(
                        div()
                            .text_sm()
                            .text_color(theme::ok())
                            .child("Address saved. Press Done to connect."),
                    );
                }
            }
        }
        col
    }

    pub fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let host = self.config.host.clone().unwrap_or_default();

        let mut models = div().flex().flex_wrap().gap_2();
        for m in AmpModel::ALL {
            let selected = self.config.model == m;
            models = models.child(
                widgets::button(("model", m as usize), m.label())
                    .bg(if selected {
                        theme::accent()
                    } else {
                        theme::surface()
                    })
                    .text_color(if selected {
                        gpui::rgb(0x000000)
                    } else {
                        theme::text()
                    })
                    .on_click(cx.listener(move |t, _, _, cx| {
                        t.set_model(m);
                        cx.notify();
                    })),
            );
        }

        let max = self.config.max_volume;
        let stepper = div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                widgets::button("max-down", "−").on_click(cx.listener(|t, _, _, cx| {
                    t.set_max_volume(t.config.max_volume as i32 - 1);
                    cx.notify();
                })),
            )
            .child(
                div()
                    .w(px(40.))
                    .flex()
                    .justify_center()
                    .child(max.to_string()),
            )
            .child(
                widgets::button("max-up", "+").on_click(cx.listener(|t, _, _, cx| {
                    t.set_max_volume(t.config.max_volume as i32 + 1);
                    cx.notify();
                })),
            );

        let mut sources = div().flex().flex_col().gap_1();
        for s in self.sources() {
            let id = s.id;
            let label_text = s.label.clone().unwrap_or_default();
            sources =
                sources.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(widgets::toggle(("en", id as usize), s.enabled).on_click(
                            cx.listener(move |t, _, _, cx| {
                                t.update_source(id, |s| s.enabled = !s.enabled);
                                cx.notify();
                            }),
                        ))
                        .child(div().w(px(80.)).text_sm().child(s.name.clone()))
                        .child(div().flex_1().child(self.text_field(
                            ("lbl", id as usize),
                            Field::Label(id),
                            label_text,
                            "label",
                            cx,
                        ))),
                );
        }

        div()
            .id("settings-scroll")
            .size_full()
            .overflow_y_scroll()
            .p_4()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_lg().child("Settings"))
                    .child(
                        widgets::button("done", "Done")
                            .on_click(cx.listener(|t, _, w, cx| t.close_settings(w, cx))),
                    ),
            )
            .child(widgets::section_title(
                "AMPLIFIER ADDRESS (host or host:port)",
            ))
            .child(self.text_field("host", Field::Host, host, "e.g. 192.168.1.111", cx))
            .child(self.render_search(cx))
            .child(widgets::section_title("AMPLIFIER TYPE"))
            .child(models)
            .child(widgets::section_title("MENU BAR"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        widgets::toggle("menu-bar", self.config.menu_bar).on_click(cx.listener(
                            |t, _, _, cx| {
                                t.toggle_menu_bar(cx);
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child("Show in menu bar (closing the window hides it)"),
                    ),
            )
            .child(widgets::section_title("MAX VOLUME"))
            .child(stepper)
            .child(widgets::section_title("SOURCES (toggle + label)"))
            .child(sources)
            .child(
                div()
                    .mt_3()
                    .child(
                        widgets::button("reset-src", "Reset sources").on_click(cx.listener(
                            |t, _, _, cx| {
                                t.reset_sources();
                                cx.notify();
                            },
                        )),
                    ),
            )
    }
}
