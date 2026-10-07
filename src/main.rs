#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod shortcut;
mod typing;

use eframe::egui::{self, Color32, RichText};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde::{Deserialize, Serialize};
use shortcut::{ShortcutConfig, KEYS};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc,
};
use typing::{TypingEvent, TypingSettings};

const STORAGE_KEY: &str = "typing_simulator_settings";
const BG: Color32 = Color32::from_rgb(12, 17, 30);
const CARD: Color32 = Color32::from_rgb(24, 32, 51);
const BORDER: Color32 = Color32::from_rgb(47, 61, 87);
const CYAN: Color32 = Color32::from_rgb(64, 216, 237);
const MUTED: Color32 = Color32::from_rgb(156, 170, 195);

fn main() -> eframe::Result {
    let icon = load_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Typing Simulator")
            .with_icon(icon)
            .with_inner_size([960.0, 740.0])
            .with_min_inner_size([720.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Typing Simulator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("embedded app icon is a valid PNG")
        .thumbnail(256, 256)
        .to_rgba8();
    egui::IconData {
        rgba: image.into_raw(),
        width: 256,
        height: 256,
    }
}

fn set_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = BG;
    style.visuals.widgets.noninteractive.bg_fill = CARD;
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(34, 44, 66);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 62, 88);
    style.visuals.selection.bg_fill = Color32::from_rgb(32, 104, 135);
    style.visuals.hyperlink_color = CYAN;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    ctx.set_style(style);
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    wpm: u32,
    delay_seconds: f32,
    typo_percent: f32,
    variation_percent: f32,
    shortcut: ShortcutConfig,
    guide_seen: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wpm: 120,
            delay_seconds: 3.0,
            typo_percent: 10.0,
            variation_percent: 15.0,
            shortcut: ShortcutConfig::default(),
            guide_seen: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    Mac,
}

struct App {
    text: String,
    config: Config,
    draft_shortcut: ShortcutConfig,
    active: Arc<AtomicBool>,
    status: String,
    status_error: bool,
    events: Option<Receiver<TypingEvent>>,
    hotkey_manager: Option<GlobalHotKeyManager>,
    registered_hotkey: Option<HotKey>,
    show_guide: bool,
    guide_step: usize,
    guide_platform: Platform,
    icon_texture: egui::TextureHandle,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        set_style(&cc.egui_ctx);
        let icon = load_icon();
        let icon_image = egui::ColorImage::from_rgba_unmultiplied(
            [icon.width as usize, icon.height as usize],
            &icon.rgba,
        );
        let icon_texture =
            cc.egui_ctx
                .load_texture("app-icon", icon_image, egui::TextureOptions::LINEAR);
        let mut config: Config = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, STORAGE_KEY))
            .unwrap_or_default();
        config.wpm = config.wpm.clamp(20, 250);
        config.delay_seconds = config.delay_seconds.clamp(0.0, 10.0);
        config.typo_percent = config.typo_percent.clamp(0.0, 25.0);
        config.variation_percent = config.variation_percent.clamp(0.0, 50.0);
        if config.shortcut.hotkey().is_err() {
            config.shortcut = ShortcutConfig::default();
        }

        let (hotkey_manager, registered_hotkey, status, status_error) =
            match GlobalHotKeyManager::new() {
                Ok(manager) => {
                    let hotkey = config.shortcut.hotkey().expect("validated shortcut");
                    match manager.register(hotkey) {
                        Ok(()) => (
                            Some(manager),
                            Some(hotkey),
                            format!("Ready · shortcut {}", config.shortcut.label()),
                            false,
                        ),
                        Err(error) => (
                            Some(manager),
                            None,
                            format!("Shortcut unavailable: {error}. The Start button still works."),
                            true,
                        ),
                    }
                }
                Err(error) => (
                    None,
                    None,
                    format!("Global shortcuts unavailable: {error}. The Start button still works."),
                    true,
                ),
            };

        Self {
            text: String::new(),
            draft_shortcut: config.shortcut,
            show_guide: !config.guide_seen,
            guide_step: 0,
            guide_platform: if cfg!(target_os = "macos") {
                Platform::Mac
            } else {
                Platform::Windows
            },
            config,
            active: Arc::new(AtomicBool::new(false)),
            status,
            status_error,
            events: None,
            hotkey_manager,
            registered_hotkey,
            icon_texture,
        }
    }

    fn toggle(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            self.active.store(false, Ordering::SeqCst);
            self.status = "Stopping…".into();
            return;
        }
        if self.text.trim().is_empty() {
            self.status = "Add some text before starting.".into();
            self.status_error = true;
            return;
        }
        // A new flag keeps a quick restart from reviving an earlier worker.
        self.active = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = mpsc::channel();
        self.events = Some(receiver);
        self.status = "Get ready to focus the target window…".into();
        self.status_error = false;
        typing::spawn(
            self.text.clone(),
            TypingSettings {
                wpm: self.config.wpm,
                delay_seconds: self.config.delay_seconds,
                typo_chance: self.config.typo_percent / 100.0,
                timing_variation: self.config.variation_percent / 100.0,
            },
            Arc::clone(&self.active),
            sender,
        );
    }

    fn apply_shortcut(&mut self) {
        let new_hotkey = match self.draft_shortcut.hotkey() {
            Ok(hotkey) => hotkey,
            Err(message) => {
                self.status = message.into();
                self.status_error = true;
                return;
            }
        };
        let Some(manager) = &self.hotkey_manager else {
            self.status = "System-wide shortcuts are unavailable on this device.".into();
            self.status_error = true;
            return;
        };
        if self.registered_hotkey == Some(new_hotkey) {
            self.config.shortcut = self.draft_shortcut;
            self.status = format!("Shortcut is already {}", self.draft_shortcut.label());
            self.status_error = false;
            return;
        }
        if let Err(error) = manager.register(new_hotkey) {
            self.status = format!(
                "Could not use {}: {error}. Your previous shortcut is still active.",
                self.draft_shortcut.label()
            );
            self.status_error = true;
            return;
        }
        if let Some(old_hotkey) = self.registered_hotkey {
            if let Err(error) = manager.unregister(old_hotkey) {
                let _ = manager.unregister(new_hotkey);
                self.status = format!("Could not replace the old shortcut: {error}");
                self.status_error = true;
                return;
            }
        }
        self.registered_hotkey = Some(new_hotkey);
        self.config.shortcut = self.draft_shortcut;
        self.status = format!("Shortcut changed to {}", self.config.shortcut.label());
        self.status_error = false;
    }

    fn poll(&mut self) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if self
                .registered_hotkey
                .is_some_and(|key| key.id() == event.id)
                && event.state == HotKeyState::Pressed
            {
                self.toggle();
            }
        }
        let pending: Vec<_> = self
            .events
            .as_ref()
            .map(|receiver| receiver.try_iter().collect())
            .unwrap_or_default();
        for event in pending {
            match event {
                TypingEvent::Countdown(seconds) => {
                    self.status = format!("Focus the target window · typing in {seconds}…")
                }
                TypingEvent::Typing => {
                    self.status = format!("Typing · press {} to stop", self.config.shortcut.label())
                }
                TypingEvent::Finished => self.status = "Finished typing".into(),
                TypingEvent::Stopped => {
                    self.active.store(false, Ordering::SeqCst);
                    self.status = "Stopped".into();
                }
                TypingEvent::Error(message) => {
                    self.status = message;
                    self.status_error = true;
                }
            }
        }
    }

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(CARD)
                    .inner_margin(egui::Margin::symmetric(22, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.image((self.icon_texture.id(), egui::vec2(48.0, 48.0)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Typing Simulator").size(24.0).strong());
                        ui.label(RichText::new("Natural typing, on your terms").color(MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if self.show_guide {
                            "Hide setup guide"
                        } else {
                            "Setup guide"
                        };
                        if ui.button(label).clicked() {
                            self.show_guide = !self.show_guide;
                            if !self.show_guide {
                                self.config.guide_seen = true;
                            }
                        }
                    });
                });
            });
    }

    fn guide(&mut self, ctx: &egui::Context) {
        if !self.show_guide {
            return;
        }
        egui::SidePanel::right("setup_guide")
            .default_width(310.0).min_width(280.0).max_width(350.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(18, 26, 43)).inner_margin(egui::Margin::same(18)))
            .show(ctx, |ui| {
                ui.heading("Get set up");
                ui.label(RichText::new(format!("Step {} of 4", self.guide_step + 1)).color(CYAN));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.guide_platform, Platform::Windows, "Windows");
                    ui.selectable_value(&mut self.guide_platform, Platform::Mac, "macOS");
                });
                ui.separator();
                let (title, body) = match self.guide_step {
                        0 => ("Welcome", "Paste the text you want to enter into the large editor. You can adjust speed and realism before each run."),
                        1 if self.guide_platform == Platform::Mac => ("Allow keyboard access", "Open System Settings → Privacy & Security → Accessibility. Turn on Typing Simulator. If it is not listed, add the app, then restart it. On first launch of an unsigned download, right-click the app and choose Open."),
                        1 => ("Windows permissions", "The app works normally in standard windows. If the target app is running as administrator, close Typing Simulator and run it as administrator too. Windows may ask you to confirm an unsigned download on first launch."),
                        2 => ("Try a safe target", "Open Notepad or another plain-text editor. Click Start typing here, then click inside that editor during the countdown. Begin with a short test sentence."),
                        _ => ("Control it anywhere", "Use the global shortcut to start or stop even when another window is focused. Choose a new key combination in the Shortcut card and click Apply. Letter keys require Ctrl, Alt, or Win/Cmd so normal typing is unaffected."),
                };
                ui.label(RichText::new(title).size(20.0).strong());
                ui.add_space(8.0);
                ui.label(RichText::new(body).size(15.0));
                if self.guide_step == 3 {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Current shortcut: {}", self.config.shortcut.label())).color(CYAN));
                }
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.guide_step > 0, egui::Button::new("Back")).clicked() { self.guide_step -= 1; }
                    if self.guide_step < 3 {
                        if ui.button("Next →").clicked() { self.guide_step += 1; }
                    } else if ui.button("Done").clicked() {
                        self.show_guide = false;
                        self.config.guide_seen = true;
                    }
                });
            });
    }

    fn main_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(20))).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let active = self.active.load(Ordering::SeqCst);
                section(ui, |ui| {
                    ui.horizontal(|ui| {
                        let color = if self.status_error { Color32::from_rgb(255, 144, 144) } else if active { CYAN } else { Color32::from_rgb(139, 224, 183) };
                        ui.colored_label(color, "●");
                        ui.label(RichText::new(&self.status).color(color).strong());
                    });
                });
                ui.add_space(14.0);
                section(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Your text").size(18.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} characters", self.text.chars().count())).color(MUTED));
                        });
                    });
                    ui.label(RichText::new("Paste once, then choose where it goes.").color(MUTED));
                    ui.add_space(4.0);
                    ui.add_enabled(!active, egui::TextEdit::multiline(&mut self.text)
                        .hint_text("Paste or type your text here…")
                        .desired_rows(9)
                        .desired_width(f32::INFINITY));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let label = if active { "Stop typing" } else { "Start typing" };
                        let button = egui::Button::new(RichText::new(label).strong().color(BG)).fill(CYAN);
                        if ui.add_sized([150.0, 38.0], button).clicked() { self.toggle(); }
                        ui.label(RichText::new(format!("or press {}", self.config.shortcut.label())).color(MUTED));
                    });
                });
                ui.add_space(14.0);
                section(ui, |ui| {
                    ui.label(RichText::new("Typing feel").size(18.0).strong());
                    ui.label(RichText::new("These settings apply to the next run.").color(MUTED));
                    ui.add_space(4.0);
                    ui.columns(2, |columns| {
                        columns[0].label("Speed");
                        columns[0].add(egui::Slider::new(&mut self.config.wpm, 20..=250).suffix(" WPM"));
                        columns[0].label("Corrected typo chance");
                        columns[0].add(egui::Slider::new(&mut self.config.typo_percent, 0.0..=25.0).suffix("%"));
                        columns[1].label("Start countdown");
                        columns[1].add(egui::Slider::new(&mut self.config.delay_seconds, 0.0..=10.0).suffix(" sec"));
                        columns[1].label("Timing variation");
                        columns[1].add(egui::Slider::new(&mut self.config.variation_percent, 0.0..=50.0).suffix("%"));
                    });
                });
                ui.add_space(14.0);
                section(ui, |ui| {
                    ui.label(RichText::new("Global shortcut").size(18.0).strong());
                    ui.label(RichText::new("Change the key that starts and stops typing from any window.").color(MUTED));
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| {
                        egui::ComboBox::from_id_salt("shortcut_key")
                            .selected_text(KEYS.get(self.draft_shortcut.key_index).map_or("Select key", |entry| entry.0))
                            .show_ui(ui, |ui| {
                                for (index, (name, _)) in KEYS.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_shortcut.key_index, index, *name);
                                }
                            });
                        ui.checkbox(&mut self.draft_shortcut.control, "Ctrl");
                        ui.checkbox(&mut self.draft_shortcut.alt, if cfg!(target_os = "macos") { "Option" } else { "Alt" });
                        ui.checkbox(&mut self.draft_shortcut.shift, "Shift");
                        ui.checkbox(&mut self.draft_shortcut.super_key, if cfg!(target_os = "macos") { "Cmd" } else { "Win" });
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Apply shortcut").clicked() { self.apply_shortcut(); }
                        ui.label(RichText::new(format!("Active: {}", self.config.shortcut.label())).color(CYAN));
                    });
                });
                ui.add_space(14.0);
                ui.label(RichText::new("Tip: Try a short sentence in Notepad or TextEdit first. The setup guide has platform-specific help.").color(MUTED));
            });
        });
    }
}

fn section(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        self.header(ctx);
        self.guide(ctx);
        self.main_content(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.config);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        if let (Some(manager), Some(hotkey)) = (&self.hotkey_manager, self.registered_hotkey) {
            let _ = manager.unregister(hotkey);
        }
    }
}
