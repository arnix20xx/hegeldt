#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod menubar;
mod ui;

use gpui::{
    App, AppContext, Bounds, KeyBinding, Menu, MenuItem, WindowBounds, WindowOptions, actions, px,
    size,
};
use gpui_platform::application;
use hegel_core::config::Config;

actions!(hegeldt, [Quit]);

fn main() {
    let menu_bar = app::config_path()
        .map(|p| Config::load_from(&p).menu_bar)
        .unwrap_or(false);

    application()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("secondary-q", Quit, None)]);
            cx.set_menus(vec![Menu {
                name: "Hegel Remote".into(),
                items: vec![MenuItem::action("Quit Hegel Remote", Quit)],
            }]);

            let bounds = Bounds::centered(None, size(px(380.), px(370.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Hegel Remote".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    // In menu bar mode the close button only hides the window.
                    window.on_window_should_close(cx, |_, _| {
                        if menubar::enabled() {
                            menubar::hide_window();
                            false
                        } else {
                            true
                        }
                    });
                    cx.new(|cx| app::RemoteApp::new(window, cx))
                },
            )
            .unwrap();
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
            if menu_bar && menubar::SUPPORTED {
                // Outside GPUI's borrow of the app, see the `menubar` docs.
                cx.spawn(async move |_| menubar::set_enabled(true)).detach();
            }
        });
}
