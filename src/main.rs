// Copyright (C) 2026 Vladimir Vecgailis.
// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod audio;
mod dsp;
mod operator;
mod simulation;
use operator::Message;
use simulation::{Record as Qso, ViewStation as Station};
mod morse;
use eframe::egui::{self, Color32, Key, RichText};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io,
    ops::RangeInclusive,
    path::Path,
    path::PathBuf,
    time::{Duration, Instant},
};

const NUMERIC_VALUE_WIDTH: f32 = 72.0;
const INITIAL_WINDOW_WIDTH: f32 = 1130.0;
const INITIAL_WINDOW_HEIGHT: f32 = 1000.0;
const WINDOW_SCREEN_HEIGHT_PADDING: f32 = 40.0;

#[derive(Default)]
struct NumericInput {
    text: String,
    editing: bool,
}

#[derive(Default)]
struct NumericInputs {
    wpm: NumericInput,
    pitch: NumericInput,
    bandwidth: NumericInput,
    rit: NumericInput,
    volume: NumericInput,
    noise: NumericInput,
    monitor: NumericInput,
    activity: NumericInput,
    minutes: NumericInput,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum StartupDensity {
    #[default]
    Regular,
    Compact,
    UltraCompact,
}

impl StartupDensity {
    fn is_compact(self) -> bool {
        self != Self::Regular
    }

    fn is_ultra_compact(self) -> bool {
        self == Self::UltraCompact
    }
}

impl NumericInputs {
    fn is_editing(&self) -> bool {
        self.wpm.editing
            || self.pitch.editing
            || self.bandwidth.editing
            || self.rit.editing
            || self.volume.editing
            || self.noise.editing
            || self.monitor.editing
            || self.activity.editing
            || self.minutes.editing
    }

    fn cancel_editing(&mut self) {
        self.wpm.editing = false;
        self.pitch.editing = false;
        self.bandwidth.editing = false;
        self.rit.editing = false;
        self.volume.editing = false;
        self.noise.editing = false;
        self.monitor.editing = false;
        self.activity.editing = false;
        self.minutes.editing = false;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
struct Settings {
    call: String,
    wpm: f32,
    pitch: f32,
    bandwidth: f32,
    rit: f32,
    volume: f32,
    minutes: u32,
    activity: usize,
    mode: usize,
    qrm: bool,
    qrn: bool,
    qsb: bool,
    flutter: bool,
    lids: bool,
    qsk: bool,
    noise: f32,
    monitor: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            call: "N0CALL".into(),
            wpm: 25.,
            pitch: 600.,
            bandwidth: 500.,
            rit: 0.,
            volume: 0.35,
            minutes: 10,
            activity: 3,
            mode: 0,
            qrm: false,
            qrn: false,
            qsb: false,
            flutter: false,
            lids: false,
            qsk: true,
            noise: 0.02,
            monitor: 0.75,
        }
    }
}
struct App {
    settings: Settings,
    numeric_inputs: NumericInputs,
    audio: Option<audio::Audio>,
    audio_error: String,
    running: bool,
    start: Instant,
    elapsed: f32,
    call: String,
    rst: String,
    nr: String,
    selected: Option<Station>,
    callers: Vec<Station>,
    call_sent: bool,
    nr_sent: bool,
    sent_call: String,
    log_revision: u64,
    last_typed_call: String,
    logs: Vec<Qso>,
    status: String,
    hint: bool,
    focus_call: bool,
    focus_nr: bool,
    session_file: PathBuf,
    data: PathBuf,
    settings_file: PathBuf,
    settings_save_pending: Option<Instant>,
    help: bool,
    startup_titlebar_configured: bool,
    startup_density: StartupDensity,
}
fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("MorseRunner")
}
fn settings_file() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(data_dir);
    base.join("morse-runner").join("settings.yaml")
}
fn write_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    let yaml = serde_saphyr::to_string(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let tmp = path.with_extension("yaml.tmp");
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    std::fs::write(&tmp, yaml)?;
    std::fs::rename(&tmp, path)
}
fn load_settings(yaml_path: &Path) -> (Settings, Option<String>) {
    match std::fs::read_to_string(yaml_path) {
        Ok(yaml) => match serde_saphyr::from_str(&yaml) {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::default(),
                Some(format!("Settings could not be loaded from YAML: {error}")),
            ),
        },
        Err(error) if error.kind() != io::ErrorKind::NotFound => (
            Settings::default(),
            Some(format!("Settings could not be read: {error}")),
        ),
        Err(_) => (Settings::default(), None),
    }
}
fn calls() -> &'static Vec<String> {
    static CALLS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    CALLS.get_or_init(|| {
        include_str!("../assets/calls.txt")
            .lines()
            .map(str::to_owned)
            .collect()
    })
}

fn numeric_slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    input: &mut NumericInput,
    id_and_label: (&'static str, &'static str),
    range: RangeInclusive<T>,
    format_value: impl Fn(T) -> String,
    parse_value: impl Fn(&str) -> Option<T>,
) {
    let (id, label) = id_and_label;
    if !input.editing {
        input.text = format_value(*value);
    }

    let row_height = (ui.text_style_height(&ui.style().drag_value_text_style)
        + 2.0 * ui.spacing().button_padding.y)
        .max(ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), row_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let size = egui::vec2(NUMERIC_VALUE_WIDTH, row_height);
            let slider = ui.add(egui::Slider::new(value, range.clone()).show_value(false));
            if slider.changed() && !input.editing {
                input.text = format_value(*value);
            }

            let field_id = ui.id().with(("numeric-value", id));
            if input.editing {
                let field = ui.add_sized(
                    size,
                    egui::TextEdit::singleline(&mut input.text)
                        .id(field_id)
                        .min_size(size)
                        .desired_width(NUMERIC_VALUE_WIDTH)
                        .horizontal_align(egui::Align::Center)
                        .vertical_align(egui::Align::Center)
                        .char_limit(6)
                        .clip_text(true)
                        .font(ui.style().drag_value_text_style.clone()),
                );
                if let Some(edited) = parse_value(&input.text) {
                    *value = if edited < *range.start() {
                        *range.start()
                    } else if edited > *range.end() {
                        *range.end()
                    } else {
                        edited
                    };
                }
                if field.lost_focus() {
                    input.editing = false;
                    input.text = format_value(*value);
                }
            } else {
                let value_button = egui::Button::new(
                    egui::RichText::new(&input.text)
                        .text_style(ui.style().drag_value_text_style.clone()),
                )
                .min_size(size);
                if ui.add_sized(size, value_button).clicked() {
                    input.editing = true;
                    ui.memory_mut(|memory| memory.request_focus(field_id));
                }
            }
            ui.label(label);
        },
    );
}

fn numeric_slider_f32(
    ui: &mut egui::Ui,
    value: &mut f32,
    input: &mut NumericInput,
    id: &'static str,
    label: &'static str,
    range: RangeInclusive<f32>,
    decimals: usize,
) {
    numeric_slider(
        ui,
        value,
        input,
        (id, label),
        range,
        |value| format!("{value:.decimals$}"),
        |text| text.parse::<f32>().ok().filter(|value| value.is_finite()),
    );
}

fn numeric_slider_usize(
    ui: &mut egui::Ui,
    value: &mut usize,
    input: &mut NumericInput,
    id: &'static str,
    label: &'static str,
    range: RangeInclusive<usize>,
) {
    numeric_slider(
        ui,
        value,
        input,
        (id, label),
        range,
        |value| value.to_string(),
        |text| text.parse().ok(),
    );
}

fn numeric_slider_u32(
    ui: &mut egui::Ui,
    value: &mut u32,
    input: &mut NumericInput,
    id: &'static str,
    label: &'static str,
    range: RangeInclusive<u32>,
) {
    numeric_slider(
        ui,
        value,
        input,
        (id, label),
        range,
        |value| value.to_string(),
        |text| text.parse().ok(),
    );
}

fn configure_ui(ctx: &egui::Context) {
    // Keep the application palette and font metrics consistent across operating systems,
    // regardless of the host OS theme. Sizes below are logical points; eframe/winit scales
    // them using the current monitor's DPI and updates that scale when the window moves.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    for (kind, size) in [
        (egui::TextStyle::Body, 16.),
        (egui::TextStyle::Button, 16.),
        (egui::TextStyle::Small, 14.),
        (egui::TextStyle::Monospace, 17.),
        (egui::TextStyle::Heading, 22.),
    ] {
        if let Some(font) = style.text_styles.get_mut(&kind) {
            font.size = size;
        }
    }
    style.spacing.item_spacing = egui::vec2(12., 9.);
    style.spacing.button_padding = egui::vec2(14., 9.);
    ctx.set_style(style);
}

fn configure_compact_ui(ctx: &egui::Context, ultra_compact: bool) {
    let mut style = (*ctx.style()).clone();
    for (kind, size) in [
        (
            egui::TextStyle::Body,
            if ultra_compact { 12.0 } else { 14.0 },
        ),
        (
            egui::TextStyle::Button,
            if ultra_compact { 12.0 } else { 14.0 },
        ),
        (
            egui::TextStyle::Small,
            if ultra_compact { 10.0 } else { 12.0 },
        ),
        (
            egui::TextStyle::Monospace,
            if ultra_compact { 13.0 } else { 15.0 },
        ),
        (
            egui::TextStyle::Heading,
            if ultra_compact { 16.0 } else { 19.0 },
        ),
    ] {
        if let Some(font) = style.text_styles.get_mut(&kind) {
            font.size = size;
        }
    }
    style.spacing.item_spacing.y = if ultra_compact { 1.0 } else { 3.0 };
    style.spacing.button_padding.y = if ultra_compact { 2.0 } else { 4.0 };
    style.spacing.interact_size.y = if ultra_compact { 20.0 } else { 24.0 };
    ctx.set_style(style);
}

#[cfg(target_os = "windows")]
fn configure_native_titlebar(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_NOZORDER,
    };

    let Ok(window_handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(window_handle) = window_handle.as_raw() else {
        return;
    };

    // SetWindowTheme in winit opts into Windows' dark non-client rendering. Setting
    // the documented DWM attribute and refreshing the frame makes it take effect
    // before the first interaction while leaving active/inactive colors to Windows.
    let dark_mode: i32 = 1;
    let hwnd = window_handle.hwnd.get() as _;
    unsafe {
        let result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark_mode as *const i32 as _,
            std::mem::size_of_val(&dark_mode) as u32,
        );
        // Windows 10 builds before 20H1 exposed the same setting at attribute 19.
        if result < 0 {
            DwmSetWindowAttribute(
                hwnd,
                19,
                &dark_mode as *const i32 as _,
                std::mem::size_of_val(&dark_mode) as u32,
            );
        }
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED
                | SWP_NOMOVE
                | SWP_NOSIZE
                | SWP_NOZORDER
                | SWP_NOOWNERZORDER
                | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn configure_native_titlebar(_frame: &eframe::Frame) {}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_ui(&cc.egui_ctx);
        let data = data_dir();
        let settings_file = settings_file();
        let (settings, settings_status) = load_settings(&settings_file);
        let (audio, audio_error) = match audio::Audio::open() {
            Ok(a) => (Some(a), String::new()),
            Err(e) => (None, e),
        };
        Self {
            settings,
            numeric_inputs: NumericInputs::default(),
            audio,
            audio_error,
            running: false,
            start: Instant::now(),
            elapsed: 0.,
            call: String::new(),
            rst: "599".into(),
            nr: String::new(),
            selected: None,
            callers: vec![],
            call_sent: false,
            nr_sent: false,
            sent_call: String::new(),
            log_revision: 0,
            last_typed_call: String::new(),
            logs: vec![],
            status: settings_status
                .unwrap_or_else(|| "Ready. Start a session, then press F1 to call CQ.".into()),
            hint: false,
            focus_call: false,
            focus_nr: false,
            session_file: data.join("session.json"),
            data,
            settings_file,
            settings_save_pending: None,
            help: false,
            startup_titlebar_configured: false,
            startup_density: StartupDensity::Regular,
        }
    }
    fn save_settings(&mut self) {
        if let Err(e) = write_settings(&self.settings_file, &self.settings) {
            self.status = format!("Settings could not be saved: {e}");
        }
    }
    fn reset_fields(&mut self) {
        self.call.clear();
        self.rst = "599".into();
        self.nr.clear();
        self.call_sent = false;
        self.nr_sent = false;
        self.sent_call.clear();
        self.last_typed_call.clear();
        self.focus_call = true;
    }
    fn config(&self) -> simulation::Config {
        simulation::Config {
            call: self.settings.call.clone(),
            wpm: self.settings.wpm,
            pitch: self.settings.pitch,
            bandwidth: self.settings.bandwidth,
            rit: self.settings.rit,
            volume: self.settings.volume,
            noise: self.settings.noise,
            monitor: self.settings.monitor,
            qsb: self.settings.qsb,
            flutter: self.settings.flutter,
            qrn: self.settings.qrn,
            qrm: self.settings.qrm,
            lids: self.settings.lids,
            qsk: self.settings.qsk,
            single: self.settings.mode == 1,
            activity: self.settings.activity,
        }
    }
    fn start_session(&mut self) {
        if self.audio.is_none() {
            self.status = format!("Audio unavailable: {}", self.audio_error);
            return;
        }
        self.settings.call = self.settings.call.trim().to_uppercase();
        if self.settings.call.is_empty() {
            self.status = "Enter MY CALL first.".into();
            return;
        }
        self.save_settings();
        self.logs.clear();
        self.reset_fields();
        self.callers.clear();
        self.selected = None;
        self.start = Instant::now();
        self.elapsed = 0.;
        self.running = true;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.session_file = self.data.join(format!("session-{ts}.json"));
        if let Some(a) = &self.audio {
            a.mixer.lock().unwrap().start(self.config());
        }
        self.status = "Session started. F1 / Enter → CQ.".into();
    }
    fn stop(&mut self) {
        self.sync_snapshot();
        self.running = false;
        if let Some(a) = &self.audio {
            a.mixer.lock().unwrap().stop();
        }
        self.status = "Session finished. Your log is saved locally.".into();
        self.save_log();
    }
    fn send(&mut self, messages: &[Message]) {
        self.sync_call_edit();
        if !self.running {
            return;
        }
        if messages.contains(&Message::HisCall) {
            self.call_sent = true;
            self.sent_call = self.call.clone();
        }
        if messages.contains(&Message::Nr) {
            self.nr_sent = true;
            self.focus_nr = true;
        }
        if let Some(a) = &self.audio {
            let mut engine = a.mixer.lock().unwrap();
            if messages.contains(&Message::Tu) {
                engine.complete_exchange(messages, &self.call);
            } else {
                engine.send(messages, &self.call);
            }
        }
    }
    fn action(&mut self, n: usize) {
        if !self.running {
            return;
        }
        match n {
            1 => self.send(&[Message::Cq]),
            2 => self.send(&[Message::Nr]),
            3 => self.complete(),
            4 => self.send(&[Message::MyCall]),
            5 => self.send(&[Message::HisCall]),
            6 => self.send(&[Message::B4]),
            7 => self.send(&[Message::Qm]),
            8 => self.send(&[Message::Nil]),
            _ => {}
        }
    }
    fn enter(&mut self) {
        self.sync_call_edit();
        if !self.running {
            return;
        }
        if self.call.trim().is_empty() {
            self.send(&[Message::Cq]);
            return;
        }
        let c = self.call_sent;
        let n = self.nr_sent;
        let r = !self.nr.is_empty();
        let mut messages = vec![];
        if !c || (!n && !r) {
            messages.push(Message::HisCall);
        }
        if !n {
            messages.push(Message::Nr);
        }
        if n && !r {
            messages.push(Message::Qm);
        }
        if r && (c || n) {
            messages.extend([Message::Tu, Message::MyCall]);
            self.send(&messages);
            self.log_entry();
        } else {
            self.send(&messages);
            self.focus_nr = true;
        }
    }
    fn complete(&mut self) {
        if !self.running {
            return;
        }
        if !self.valid_entry() {
            return;
        }
        let mut messages = vec![];
        if !self.call_sent {
            messages.push(Message::HisCall);
        }
        messages.extend([Message::Tu, Message::MyCall]);
        self.send(&messages);
        self.log_entry();
    }
    fn valid_entry(&mut self) -> bool {
        if self.call.trim().len() < 3 || self.rst.len() != 3 || self.nr.is_empty() {
            self.status =
                "Enter callsign, three-digit RST and received serial before logging.".into();
            return false;
        }
        true
    }
    fn log_entry(&mut self) {
        if !self.valid_entry() {
            return;
        }
        if let Some(a) = &self.audio {
            a.mixer.lock().unwrap().log(&self.call, &self.rst, &self.nr);
        }
        self.sync_snapshot();
        self.reset_fields();
        self.save_log();
    }
    fn sync_snapshot(&mut self) {
        let snapshot = self
            .audio
            .as_ref()
            .map(|a| a.mixer.lock().unwrap().snapshot());
        if let Some(s) = snapshot {
            self.elapsed = s.elapsed;
            self.selected = s
                .callers
                .iter()
                .find(|st| {
                    st.call == self.sent_call
                        || matches!(
                            st.state,
                            operator::State::NeedNr
                                | operator::State::NeedCall
                                | operator::State::NeedCallNr
                                | operator::State::NeedEnd
                        )
                })
                .cloned();
            self.callers = s.callers;
            if self.running {
                self.status = if s.transmitting {
                    format!("TX · {}", s.text)
                } else {
                    format!("RX · {} stations on frequency", self.callers.len())
                };
            }
            if self.log_revision != s.revision {
                self.log_revision = s.revision;
                self.logs = s.logs;
                if !self.logs.is_empty() {
                    self.save_log();
                }
            }
        }
    }
    fn save_log(&mut self) {
        if let Err(e) = std::fs::create_dir_all(&self.data).and_then(|_| {
            std::fs::write(
                &self.session_file,
                serde_json::to_vec_pretty(&self.logs).unwrap(),
            )
        }) {
            self.status = format!("Could not save log: {e}");
        }
    }
    fn export(&mut self) {
        let p = self.session_file.with_extension("csv");
        let mut s =
            "Elapsed,Call,RST,Received,Sent,ActualCall,ActualRST,ActualReceived,Error\n".to_owned();
        for q in &self.logs {
            s.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                q.time, q.call, q.rst, q.nr, q.sent, q.true_call, q.true_rst, q.true_nr, q.error
            ));
        }
        match std::fs::write(&p, s) {
            Ok(_) => self.status = format!("Exported {}", p.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }
    fn sync_call_edit(&mut self) {
        if self.call == self.last_typed_call {
            return;
        }
        if self.call.is_empty() {
            self.nr_sent = false;
        }
        if let Some(a) = &self.audio {
            if a.mixer.lock().unwrap().update_call(&self.call) {
                self.sent_call = self.call.clone();
            } else {
                self.call_sent = false;
            }
        }
        self.last_typed_call = self.call.clone();
    }
    fn update_engine(&mut self) {
        let config = self.config();
        if let Some(a) = &self.audio {
            if let Some(err) = a.error.lock().unwrap().take() {
                self.audio_error = err;
                self.running = false;
            }
            let mut m = a.mixer.lock().unwrap();
            m.config = config;
            m.active = self.running;
        }
        if self.running {
            self.sync_snapshot();
            if self.elapsed >= self.settings.minutes as f32 * 60. {
                self.stop();
            }
        }
    }
}

#[cfg(test)]
mod ui_style_tests {
    use super::*;

    #[test]
    fn uses_dark_palette_and_shared_logical_font_sizes_even_when_windows_prefers_light() {
        let ctx = egui::Context::default();
        configure_ui(&ctx);

        ctx.begin_pass(egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        });
        let _ = ctx.end_pass();

        assert_eq!(ctx.theme(), egui::Theme::Dark);
        let style = ctx.style();
        assert!(style.visuals.dark_mode);
        assert_eq!(style.text_styles[&egui::TextStyle::Body].size, 16.);
        assert_eq!(style.text_styles[&egui::TextStyle::Button].size, 16.);
        assert_eq!(style.text_styles[&egui::TextStyle::Small].size, 14.);
        assert_eq!(style.text_styles[&egui::TextStyle::Monospace].size, 17.);
        assert_eq!(style.text_styles[&egui::TextStyle::Heading].size, 22.);
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.startup_titlebar_configured {
            configure_native_titlebar(frame);
            self.startup_titlebar_configured = true;

            let height = ctx
                .input(|input| input.viewport().monitor_size.map(|size| size.y))
                .map(|monitor_height| {
                    (monitor_height - WINDOW_SCREEN_HEIGHT_PADDING)
                        .clamp(320.0, INITIAL_WINDOW_HEIGHT)
                })
                .unwrap_or(INITIAL_WINDOW_HEIGHT);
            self.startup_density = if height <= 680.0 {
                StartupDensity::UltraCompact
            } else if height < INITIAL_WINDOW_HEIGHT {
                StartupDensity::Compact
            } else {
                StartupDensity::Regular
            };
            if self.startup_density.is_compact() {
                configure_compact_ui(ctx, self.startup_density.is_ultra_compact());
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                INITIAL_WINDOW_WIDTH,
                height,
            )));
        }
        let settings_before = self.settings.clone();
        self.update_engine();
        ctx.request_repaint_after(Duration::from_millis(40));
        let keys = [
            Key::F1,
            Key::F2,
            Key::F3,
            Key::F4,
            Key::F5,
            Key::F6,
            Key::F7,
            Key::F8,
        ];
        let numeric_input_active = self.numeric_inputs.is_editing();
        // Process ESM after TextEdit consumes this frame's text events, so
        // a fast final character + Enter cannot transmit/log a truncated value.
        let pending_actions: Vec<_> = if numeric_input_active {
            Vec::new()
        } else {
            keys.iter()
                .enumerate()
                .filter_map(|(n, k)| {
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, *k))
                        .then_some(n + 1)
                })
                .collect()
        };
        let enter_pressed = !numeric_input_active
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        let ctrl_enter_pressed = !numeric_input_active
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, Key::Enter));
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            if numeric_input_active {
                self.numeric_inputs.cancel_editing();
            } else {
                if let Some(a) = &self.audio {
                    a.mixer.lock().unwrap().abort_tx();
                }
                self.call_sent = false;
                self.nr_sent = false;
                self.status = "Transmission cancelled.".into();
            }
        }
        if !numeric_input_active
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F11))
        {
            self.reset_fields();
        }
        if !numeric_input_active
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::PageUp))
        {
            self.settings.wpm = (self.settings.wpm + 5.).min(80.);
        }
        if !numeric_input_active
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::PageDown))
        {
            self.settings.wpm = (self.settings.wpm - 5.).max(10.);
        }
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(
                    20,
                    if self.startup_density.is_compact() {
                        6
                    } else {
                        12
                    },
                )),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("MORSE RUNNER").color(Color32::from_rgb(89, 223, 178)),
                    );
                    ui.label(
                        RichText::new(format!("v{}", env!("MORSE_RUNNER_BUILD_VERSION"))).small(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized(
                                [
                                    64.,
                                    if self.startup_density.is_compact() {
                                        32.
                                    } else {
                                        40.
                                    },
                                ],
                                egui::Button::new("Help"),
                            )
                            .clicked()
                        {
                            self.help = true;
                        }
                        let live_color = Color32::from_rgb(255, 87, 105);
                        let color = if self.running {
                            live_color
                        } else {
                            Color32::from_gray(145)
                        };
                        let badge = ui.add_sized(
                            [
                                104.,
                                if self.startup_density.is_compact() {
                                    32.
                                } else {
                                    40.
                                },
                            ],
                            egui::Button::new(
                                RichText::new(if self.running {
                                    "    LIVE"
                                } else {
                                    "    STANDBY"
                                })
                                .strong()
                                .color(color),
                            )
                            .fill(if self.running {
                                Color32::from_rgb(58, 24, 32)
                            } else {
                                Color32::from_rgb(29, 31, 34)
                            })
                            .stroke(egui::Stroke::new(
                                1_f32,
                                if self.running {
                                    live_color
                                } else {
                                    Color32::from_gray(65)
                                },
                            ))
                            .corner_radius(8.)
                            .sense(egui::Sense::hover()),
                        );
                        ui.painter().circle_filled(
                            egui::pos2(badge.rect.left() + 17., badge.rect.center().y),
                            4.,
                            color,
                        );
                    });
                });
            });
        let sidebar_top_margin = if self.startup_density.is_compact() {
            6
        } else {
            12
        };
        let sidebar_bottom_margin = if self.startup_density.is_compact() {
            6
        } else {
            16
        };
        egui::SidePanel::left("settings")
            .exact_width(300.)
            .resizable(false)
            .show_separator_line(true)
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                    left: 20,
                    right: 20,
                    top: sidebar_top_margin,
                    bottom: sidebar_bottom_margin,
                }),
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    let session_button_height =
                        if self.startup_density.is_compact() { 32. } else { 40. };
                    let session_button = if self.running {
                        egui::Button::new("■  STOP SESSION")
                    } else {
                        egui::Button::new("▶  START SESSION")
                            .fill(Color32::from_rgb(24, 108, 83))
                    };
                    if ui
                        .add_sized(
                            [ui.available_width(), session_button_height],
                            session_button,
                        )
                        .clicked()
                    {
                        if self.running {
                            self.stop();
                        } else {
                            self.start_session();
                        }
                    }
                    ui.add_space(if self.startup_density.is_compact() {
                        6.
                    } else {
                        12.
                    });

                    let max_scroll_height = ui.available_height();
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings-scroll")
                            .max_height(max_scroll_height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                            let top_space = if self.startup_density.is_ultra_compact() {
                                2.
                            } else if self.startup_density.is_compact() {
                                4.
                            } else {
                                12.
                            };
                            ui.add_space(top_space);
                            ui.heading("STATION");
                            ui.label("MY CALL");
                            ui.add_enabled(
                                !self.running,
                                egui::TextEdit::singleline(&mut self.settings.call)
                                    .desired_width(220.),
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.wpm,
                                &mut self.numeric_inputs.wpm,
                                "wpm",
                                "WPM",
                                10.0..=80.0,
                                0,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.pitch,
                                &mut self.numeric_inputs.pitch,
                                "pitch",
                                "Pitch Hz",
                                300.0..=1000.0,
                                0,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.bandwidth,
                                &mut self.numeric_inputs.bandwidth,
                                "bandwidth",
                                "BW Hz",
                                100.0..=1000.0,
                                0,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.rit,
                                &mut self.numeric_inputs.rit,
                                "rit",
                                "RIT Hz",
                                -300.0..=300.0,
                                0,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.volume,
                                &mut self.numeric_inputs.volume,
                                "volume",
                                "Volume",
                                0.0..=1.0,
                                2,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.noise,
                                &mut self.numeric_inputs.noise,
                                "noise",
                                "Noise",
                                0.0..=0.12,
                                2,
                            );
                            numeric_slider_f32(
                                ui,
                                &mut self.settings.monitor,
                                &mut self.numeric_inputs.monitor,
                                "monitor",
                                "Sidetone",
                                0.0..=1.0,
                                2,
                            );
                            ui.checkbox(
                                &mut self.settings.qsk,
                                "QSK · receive between CW marks",
                            );
                            ui.separator();
                            ui.heading("BAND CONDITIONS");
                            if ui
                                .button("Realistic band")
                                .on_hover_text(
                                    "Enable the original QRM, QRN, QSB, flutter and imperfect operators",
                                )
                                .clicked()
                            {
                                self.settings.qrm = true;
                                self.settings.qrn = true;
                                self.settings.qsb = true;
                                self.settings.flutter = true;
                                self.settings.lids = true;
                            }
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.settings.qrm, "QRM");
                                ui.checkbox(&mut self.settings.qrn, "QRN");
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.settings.qsb, "QSB");
                                ui.checkbox(&mut self.settings.flutter, "Flutter");
                            });
                            ui.checkbox(&mut self.settings.lids, "LIDS · imperfect operators");
                            numeric_slider_usize(
                                ui,
                                &mut self.settings.activity,
                                &mut self.numeric_inputs.activity,
                                "activity",
                                "Activity",
                                1..=8,
                            );
                            ui.separator();
                            ui.heading("SESSION");
                            ui.add_enabled_ui(!self.running, |ui| {
                                egui::ComboBox::from_label("Mode")
                                    .selected_text(
                                        ["Pile-up", "Single calls"][self.settings.mode.min(1)],
                                    )
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.settings.mode, 0, "Pile-up");
                                        ui.selectable_value(
                                            &mut self.settings.mode,
                                            1,
                                            "Single calls",
                                        );
                                    });
                                numeric_slider_u32(
                                    ui,
                                    &mut self.settings.minutes,
                                    &mut self.numeric_inputs.minutes,
                                    "minutes",
                                    "Minutes",
                                    1..=60,
                                );
                            });
                            let learning_aid_space =
                                if self.startup_density.is_ultra_compact() {
                                    2.
                                } else if self.startup_density.is_compact() {
                                    4.
                                } else {
                                    8.
                                };
                            ui.add_space(learning_aid_space);
                            ui.checkbox(&mut self.hint, "Learning aid · reveal station");
                        });
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(
                    20,
                    if self.startup_density.is_compact() {
                        4
                    } else {
                        8
                    },
                )),
            )
            .show(ctx, |ui| {
                ui.label(&self.status);
                if !self.audio_error.is_empty() {
                    ui.colored_label(Color32::LIGHT_RED, &self.audio_error);
                }
                ui.small(format!(
                    "Audio: {}  ·  Logs: {}",
                    self.audio
                        .as_ref()
                        .map(|a| a.device.as_str())
                        .unwrap_or("unavailable"),
                    self.data.display()
                ));
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(20, 12)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.);
                let good: Vec<_> = self.logs.iter().filter(|q| q.error.is_empty()).collect();
                let mult: HashSet<_> = good.iter().map(|q| q.prefix.as_str()).collect();
                let score = good.len() * mult.len();
                let left = (self.settings.minutes as f32 * 60. - self.elapsed).max(0.) as u32;
                ui.columns(4, |cols| {
                    for (col, title, value) in [
                        (0, "TIME LEFT", format!("{:02}:{:02}", left / 60, left % 60)),
                        (1, "VERIFIED QSO", good.len().to_string()),
                        (2, "PREFIXES", mult.len().to_string()),
                        (3, "SCORE", score.to_string()),
                    ] {
                        cols[col].label(RichText::new(title).small().weak());
                        cols[col].heading(RichText::new(value).size(30.).monospace());
                    }
                });
                ui.add_space(12.);
                ui.separator();
                ui.label(RichText::new("RECEIVER / STATION ACTIVITY").small().weak());
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 86.),
                    egui::Sense::hover(),
                );
                let p = ui.painter();
                p.rect_filled(rect, 5., Color32::from_rgb(12, 22, 27));
                for i in 0..9 {
                    let x = rect.left() + rect.width() * i as f32 / 8.;
                    p.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(1_f32, Color32::from_gray(35)),
                    );
                }
                if self.running {
                    for s in &self.callers {
                        let x = rect.center().x
                            + (s.pitch - self.settings.pitch - self.settings.rit) / 600.
                                * rect.width();
                        let h = if s.sending {
                            (35. + 12. * (self.elapsed * 4. + s.pitch).sin()) * s.amp
                        } else {
                            3.
                        };
                        p.line_segment(
                            [
                                egui::pos2(x, rect.bottom() - 5.),
                                egui::pos2(x, rect.bottom() - h - 20.),
                            ],
                            egui::Stroke::new(3_f32, Color32::from_rgb(73, 211, 163)),
                        );
                    }
                }
                ui.label(
                    RichText::new("Schematic activity display · not a measured spectrum")
                        .small()
                        .weak(),
                );
                ui.add_space(8.);
                ui.columns(3, |c| {
                    c[0].label("CALLSIGN");
                    let r = c[0].add(
                        egui::TextEdit::singleline(&mut self.call).font(egui::TextStyle::Heading),
                    );
                    if self.focus_call {
                        r.request_focus();
                        self.focus_call = false;
                    }
                    c[1].label("RST");
                    c[1].add(
                        egui::TextEdit::singleline(&mut self.rst).font(egui::TextStyle::Heading),
                    );
                    c[2].label("RECEIVED SERIAL");
                    let r = c[2].add(
                        egui::TextEdit::singleline(&mut self.nr).font(egui::TextStyle::Heading),
                    );
                    if self.focus_nr {
                        r.request_focus();
                        self.focus_nr = false;
                    }
                });
                self.call = self
                    .call
                    .to_uppercase()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '?')
                    .take(20)
                    .collect();
                self.rst.retain(|c| c.is_ascii_digit());
                self.rst.truncate(3);
                self.nr.retain(|c| c.is_ascii_digit());
                self.nr.truncate(5);
                ui.add_space(6.);
                ui.horizontal_wrapped(|ui| {
                    for (i, label) in [
                        "F1  CQ",
                        "F2  599 #",
                        "F3  TU + Log",
                        "F4  My call",
                        "F5  His call",
                        "F6  B4",
                        "F7  ?",
                        "F8  NIL",
                    ]
                    .iter()
                    .enumerate()
                    {
                        if ui.button(*label).clicked() {
                            self.action(i + 1);
                        }
                    }
                });
                if self.hint {
                    let s = self.selected.as_ref().or(self.callers.first());
                    if let Some(s) = s {
                        ui.colored_label(
                            Color32::from_rgb(239, 196, 110),
                            format!("Learning aid: {} · {} · {:03}", s.call, s.rst, s.nr),
                        );
                    }
                }
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.heading("LOGBOOK");
                    ui.label(format!(
                        "{} total · {} errors",
                        self.logs.len(),
                        self.logs.iter().filter(|q| !q.error.is_empty()).count()
                    ));
                    if ui.button("Export CSV").clicked() {
                        self.export();
                    }
                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("log")
                            .striped(true)
                            .min_col_width(((ui.available_width() * 0.72 - 60.) / 6.).max(82.))
                            .show(ui, |ui| {
                                for t in ["Elapsed", "Call", "RST", "Rcvd", "Sent", "Check"] {
                                    ui.strong(t);
                                }
                                ui.end_row();
                                for q in self.logs.iter().rev() {
                                    ui.monospace(&q.time);
                                    ui.monospace(&q.call);
                                    ui.monospace(&q.rst);
                                    ui.monospace(&q.nr);
                                    ui.monospace(format!("{:03}", q.sent));
                                    ui.colored_label(
                                        if q.error.is_empty() {
                                            Color32::LIGHT_GREEN
                                        } else {
                                            Color32::LIGHT_RED
                                        },
                                        if q.error.is_empty() { "OK" } else { &q.error },
                                    );
                                    ui.end_row();
                                }
                            });
                    });
            });
        self.sync_call_edit();
        for action in pending_actions {
            self.action(action);
        }
        if enter_pressed {
            self.enter();
        }
        if ctrl_enter_pressed {
            self.complete();
        }
        if self.help {
            egui::Window::new("Operating guide").open(&mut self.help).show(ctx,|ui|{ui.label("1. Set your callsign, speed and conditions. Start session.\n2. F1 / Enter calls CQ (single-call mode starts automatically).\n3. Copy a caller into CALLSIGN. Enter sends their call and 599 + serial.\n4. Copy their report and number. Enter sends TU and logs the QSO.\n5. F7 repeats; Escape cancels transmission; F11 clears entry.\nPage Up / Down changes speed. Ctrl+Enter sends TU and logs.\nUse Learning aid to check the caller while practicing.\nLogs are saved automatically; CSV exports next to the JSON log.\n\nOperator and propagation algorithms ported from the Delphi source.\nWPX/HST competition modes remain outside this practice-mode release.");});
        }
        if self.settings != settings_before {
            self.settings_save_pending = Some(Instant::now() + Duration::from_millis(500));
        }
        if self
            .settings_save_pending
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.save_settings();
            self.settings_save_pending = None;
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.save_settings();
        if !self.logs.is_empty() {
            self.save_log();
        }
    }
}
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--audio-check") {
        match audio::Audio::open() {
            Ok(a) => {
                println!("Audio device: {}", a.device);
                a.mixer.lock().unwrap().active = true;
                std::thread::sleep(Duration::from_millis(300));
                if let Some(e) = a.error.lock().unwrap().as_ref() {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                println!("Audio stream OK");
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT])
            .with_min_inner_size([1000., 320.])
            .with_clamp_size_to_monitor_size(true),
        ..Default::default()
    };
    eframe::run_native(
        "Morse Runner",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn paths() -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "morse-runner-settings-{}-{unique}",
            std::process::id()
        ));
        (dir.join("settings.yaml"), dir)
    }

    #[test]
    fn loads_yaml_settings() {
        let (yaml_path, dir) = paths();
        std::fs::create_dir_all(&dir).unwrap();
        let expected = Settings {
            call: "W1AW".into(),
            wpm: 31.0,
            qrm: true,
            ..Settings::default()
        };
        write_settings(&yaml_path, &expected).unwrap();

        let (settings, status) = load_settings(&yaml_path);

        assert_eq!(settings, expected);
        assert!(status.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_yaml_uses_defaults_and_ignores_json() {
        let (yaml_path, dir) = paths();
        std::fs::create_dir_all(&dir).unwrap();
        let json_path = dir.join("settings.json");
        let legacy = br#"{"call":"W1AW","wpm":31.0,"qrm":true}"#;
        std::fs::write(&json_path, legacy).unwrap();

        let (settings, status) = load_settings(&yaml_path);

        assert_eq!(settings, Settings::default());
        assert!(status.is_none());
        assert!(!yaml_path.exists());
        assert_eq!(std::fs::read(&json_path).unwrap(), legacy);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_yaml_uses_defaults_and_reports_error() {
        let (yaml_path, dir) = paths();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&yaml_path, "invalid: [").unwrap();

        let (settings, status) = load_settings(&yaml_path);

        assert_eq!(settings, Settings::default());
        assert!(status.unwrap().contains("could not be loaded from YAML"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
