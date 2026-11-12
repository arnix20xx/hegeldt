use gpui::{Context, div, prelude::*, px};
use hegeldt::model::AmpModel;

use super::{theme, widgets};
use crate::app::{Field, RemoteApp};

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
                            .on_click(cx.listener(|t, _, _, cx| t.close_settings(cx))),
                    ),
            )
            .child(widgets::section_title(
                "AMPLIFIER ADDRESS (host or host:port)",
            ))
            .child(self.text_field("host", Field::Host, host, "e.g. 192.168.1.111", cx))
            .child(
                div().mt_2().child(
                    widgets::button("search", "Search for amplifiers (not enabled yet)")
                        .text_color(theme::muted()),
                ),
            )
            .child(widgets::section_title("AMPLIFIER TYPE"))
            .child(models)
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
