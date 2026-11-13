//! Application state and behaviour; rendering lives in `ui/`.

use std::path::PathBuf;
use std::time::Duration;

use futures::StreamExt;
use gpui::{Context, FocusHandle, KeyDownEvent, Task, Window, div, prelude::*, px, size};

use hegel_core::config::Config;
use hegel_core::connection::{AmpConnection, AmpEvent};
use hegel_core::discovery::{self, DiscoveredDevice};
use hegel_core::model::{AmpModel, InputSource};
use hegel_core::protocol::{AMP_PORT, Command, Param};

use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Remote,
    Settings,
}

/// Window sizes (logical px) for the two screens.
const REMOTE_SIZE: (f32, f32) = (380., 370.);
const SETTINGS_SIZE: (f32, f32) = (380., 640.);

/// Progress of an SSDP search started from the settings screen.
#[derive(Clone, PartialEq)]
pub enum SearchState {
    Idle,
    Searching,
    Found(Vec<DiscoveredDevice>),
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Host,
    Label(u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
}

pub struct RemoteApp {
    pub config: Config,
    config_path: Option<PathBuf>,
    pub screen: Screen,

    // amplifier state
    pub connected: bool,
    pub power: bool,
    pub mute: bool,
    pub volume: u8,
    pub input: Option<u8>,
    pub message: Option<String>,

    // remote screen
    pub dropdown_open: bool,

    // amplifier search (settings)
    pub search: SearchState,
    /// Host of the device picked from the search results, for highlighting + feedback.
    pub picked: Option<String>,
    search_task: Option<Task<()>>,

    // plumbing
    conn: Option<AmpConnection>,
    event_task: Option<Task<()>>,
    reconnect_task: Option<Task<()>>,
    hold_task: Option<Task<()>>,
    blink_task: Option<Task<()>>,
    pub blink_on: bool,

    // text editing in settings
    pub editing: Option<Field>,
    pub buffer: String,

    pub focus: FocusHandle,
}

/// `$HEGELDT_CONFIG` if set, else the default location.
pub fn config_path() -> Option<PathBuf> {
    std::env::var_os("HEGELDT_CONFIG")
        .map(PathBuf::from)
        .or_else(Config::default_path)
}

/// Splits "host" or "host:port".
fn split_host_port(s: &str) -> (String, u16) {
    let s = s.trim();
    if let Some((h, p)) = s.rsplit_once(':')
        && let Ok(p) = p.parse::<u16>()
        && !h.contains(':')
    {
        return (h.to_string(), p);
    }
    (s.to_string(), AMP_PORT)
}

impl RemoteApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config_path = config_path();
        let config = config_path
            .as_deref()
            .map(Config::load_from)
            .unwrap_or_default();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let has_host = config.host.as_ref().is_some_and(|h| !h.trim().is_empty());
        let mut app = RemoteApp {
            config,
            config_path,
            screen: if has_host {
                Screen::Remote
            } else {
                Screen::Settings
            },
            connected: false,
            power: false,
            mute: false,
            volume: 0,
            input: None,
            message: None,
            dropdown_open: false,
            search: SearchState::Idle,
            picked: None,
            search_task: None,
            conn: None,
            event_task: None,
            reconnect_task: None,
            hold_task: None,
            blink_task: None,
            blink_on: true,
            editing: None,
            buffer: String::new(),
            focus,
        };
        if has_host {
            app.connect(cx);
        }
        app
    }

    pub fn sources(&self) -> Vec<InputSource> {
        self.config.inputs_for(self.config.model)
    }

    fn save(&self) {
        if let Some(p) = &self.config_path
            && let Err(e) = self.config.save_to(p)
        {
            log::warn!("could not save config: {e}");
        }
    }

    // ---- connection ---------------------------------------------------

    pub fn connect(&mut self, cx: &mut Context<Self>) {
        self.conn = None;
        self.event_task = None;
        self.reconnect_task = None;
        self.connected = false;
        self.power = false;
        self.mute = false;
        self.input = None;
        let Some(host) = self.config.host.clone().filter(|h| !h.trim().is_empty()) else {
            self.message = Some("No amplifier address set".into());
            cx.notify();
            return;
        };
        let (host, port) = split_host_port(&host);
        self.message = Some(format!("Connecting to {host}…"));
        let (conn, mut rx) = AmpConnection::connect(host, port);
        self.conn = Some(conn);
        self.event_task = Some(cx.spawn(async move |this, cx| {
            while let Some(ev) = rx.next().await {
                if this.update(cx, |t, cx| t.on_event(ev, cx)).is_err() {
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn on_event(&mut self, ev: AmpEvent, cx: &mut Context<Self>) {
        match ev {
            AmpEvent::Connected => {
                self.connected = true;
                self.message = None;
            }
            AmpEvent::Status(s) => match s.cmd {
                Command::Power => {
                    self.power = s.value == 1;
                    if !self.power {
                        self.mute = false;
                        self.input = None;
                    }
                }
                Command::Mute => {
                    self.mute = s.value == 1;
                    self.update_blink(cx);
                }
                Command::Volume => self.volume = s.value.min(99),
                Command::Input => self.input = Some(s.value),
                Command::Reset => {}
            },
            AmpEvent::Disconnected { reason } => {
                self.connected = false;
                self.message = Some(reason);
                self.conn = None;
                self.schedule_reconnect(cx);
            }
        }
        cx.notify();
    }

    fn schedule_reconnect(&mut self, cx: &mut Context<Self>) {
        self.reconnect_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            this.update(cx, |t, cx| {
                if t.screen == Screen::Remote {
                    t.connect(cx);
                }
            })
            .ok();
        }));
    }

    fn update_blink(&mut self, cx: &mut Context<Self>) {
        self.blink_on = true;
        if self.mute {
            self.blink_task = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(500))
                        .await;
                    if this
                        .update(cx, |t, cx| {
                            t.blink_on = !t.blink_on;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }));
        } else {
            self.blink_task = None;
        }
    }

    fn send(&self, cmd: Command, param: Param) {
        if let Some(c) = &self.conn {
            c.send(cmd, param);
        }
    }

    // ---- actions ------------------------------------------------------

    pub fn toggle_power(&mut self) {
        self.send(Command::Power, Param::Toggle);
    }

    pub fn toggle_mute(&mut self) {
        self.send(Command::Mute, Param::Toggle);
    }

    pub fn select_input(&mut self, id: u8) {
        self.send(Command::Input, Param::Value(id));
    }

    /// One volume step, respecting the configured maximum.
    pub fn volume_step(&mut self, dir: Dir) {
        match dir {
            Dir::Up if self.volume >= self.config.max_volume => {}
            Dir::Up => self.send(Command::Volume, Param::Up),
            Dir::Down => self.send(Command::Volume, Param::Down),
        }
    }

    /// Start auto-repeat: first repeat after 100 ms, then every 300 ms (as the Android app).
    pub fn start_hold(&mut self, dir: Dir, cx: &mut Context<Self>) {
        self.volume_step(dir);
        self.hold_task = Some(cx.spawn(async move |this, cx| {
            let mut delay = 100;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                delay = 300;
                if this.update(cx, |t, _| t.volume_step(dir)).is_err() {
                    break;
                }
            }
        }));
    }

    pub fn stop_hold(&mut self) {
        self.hold_task = None;
    }

    // ---- settings -----------------------------------------------------

    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.screen = Screen::Settings;
        self.dropdown_open = false;
        self.reconnect_task = None;
        self.stop_hold();
        window.resize(size(px(SETTINGS_SIZE.0), px(SETTINGS_SIZE.1)));
        cx.notify();
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_edit();
        self.screen = Screen::Remote;
        self.save();
        window.resize(size(px(REMOTE_SIZE.0), px(REMOTE_SIZE.1)));
        self.connect(cx);
    }

    pub fn toggle_menu_bar(&mut self, cx: &mut Context<Self>) {
        self.config.menu_bar = !self.config.menu_bar;
        let on = self.config.menu_bar;
        // Must run outside the current GPUI update, see `menubar` docs.
        cx.spawn(async move |_, _| crate::menubar::set_enabled(on))
            .detach();
        self.save();
    }

    // ---- amplifier search ---------------------------------------------

    /// Runs SSDP discovery on a background thread and shows the result in the settings.
    pub fn start_search(&mut self, cx: &mut Context<Self>) {
        if self.search == SearchState::Searching {
            return;
        }
        self.search = SearchState::Searching;
        self.picked = None;
        let job = cx
            .background_executor()
            .spawn(async move { discovery::search(Duration::from_secs(4)) });
        self.search_task = Some(cx.spawn(async move |this, cx| {
            let result = job.await;
            this.update(cx, |t, cx| {
                t.search = match result {
                    Ok(devs) => SearchState::Found(devs),
                    Err(e) => SearchState::Failed(format!("Search failed: {e}")),
                };
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Uses a search result: sets the host (and the model if it is recognised) and saves.
    pub fn pick_device(&mut self, dev: &DiscoveredDevice) {
        self.commit_edit();
        self.config.apply_device(dev);
        self.picked = Some(dev.host.clone());
        self.save();
    }

    pub fn begin_edit(&mut self, field: Field) {
        self.commit_edit();
        self.buffer = match field {
            Field::Host => self.config.host.clone().unwrap_or_default(),
            Field::Label(id) => self
                .sources()
                .iter()
                .find(|s| s.id == id)
                .and_then(|s| s.label.clone())
                .unwrap_or_default(),
        };
        self.editing = Some(field);
    }

    pub fn commit_edit(&mut self) {
        let Some(field) = self.editing.take() else {
            return;
        };
        let text = self.buffer.trim().to_string();
        match field {
            Field::Host => {
                self.config.host = (!text.is_empty()).then_some(text);
            }
            Field::Label(id) => {
                self.update_source(id, |s| s.label = (!text.is_empty()).then_some(text));
            }
        }
        self.save();
    }

    pub fn update_source(&mut self, id: u8, f: impl FnOnce(&mut InputSource)) {
        let model = self.config.model;
        let mut list = self.config.inputs_for(model);
        if let Some(s) = list.iter_mut().find(|s| s.id == id) {
            f(s);
        }
        self.config.inputs.insert(model, list);
        self.save();
    }

    pub fn set_max_volume(&mut self, v: i32) {
        self.config.max_volume = v.clamp(0, 99) as u8;
        self.save();
    }

    pub fn set_model(&mut self, model: AmpModel) {
        self.config.model = model;
        self.save();
    }

    pub fn reset_sources(&mut self) {
        self.config.inputs.remove(&self.config.model);
        self.save();
    }

    // ---- keyboard -----------------------------------------------------

    fn on_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let key = ks.key.as_str();

        // Cmd on macOS, Ctrl elsewhere.
        if ks.modifiers.secondary() {
            match key {
                "," => {
                    if self.screen == Screen::Remote {
                        self.open_settings(w, cx);
                    } else {
                        self.close_settings(w, cx);
                    }
                }
                "v" if self.editing.is_some() => {
                    if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                        self.buffer.push_str(t.lines().next().unwrap_or(""));
                    }
                }
                _ => {}
            }
            cx.notify();
            return;
        }

        if self.editing.is_some() {
            match key {
                "enter" => self.commit_edit(),
                "escape" => self.editing = None,
                "backspace" => {
                    self.buffer.pop();
                }
                _ => {
                    if !ks.modifiers.control
                        && let Some(c) = &ks.key_char
                    {
                        self.buffer.push_str(c);
                    }
                }
            }
            cx.notify();
            return;
        }

        if self.screen != Screen::Remote {
            return;
        }
        match key {
            "escape" => self.dropdown_open = false,
            "up" => self.volume_step(Dir::Up),
            "down" => self.volume_step(Dir::Down),
            "m" => self.toggle_mute(),
            "p" => self.toggle_power(),
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() => {
                let n = (d.as_bytes()[0] - b'0') as usize;
                if let Some(s) = self
                    .sources()
                    .iter()
                    .filter(|s| s.enabled)
                    .nth(n.wrapping_sub(1))
                {
                    self.select_input(s.id);
                    self.dropdown_open = false;
                }
            }
            _ => {}
        }
        cx.notify();
    }
}

impl Render for RemoteApp {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.screen {
            Screen::Remote => self.render_remote(w, cx).into_any_element(),
            Screen::Settings => self.render_settings(cx).into_any_element(),
        };
        div()
            .id("root")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .child(body)
    }
}
