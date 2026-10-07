#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod typing;

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc,
};
use typing::{TypingEvent, TypingSettings};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Typing Simulator")
            .with_inner_size([620.0, 610.0])
            .with_min_inner_size([480.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Typing Simulator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    text: String,
    wpm: u32,
    delay_seconds: f32,
    typo_percent: f32,
    variation_percent: f32,
    active: Arc<AtomicBool>,
    status: String,
    events: Option<Receiver<TypingEvent>>,
    _hotkey_manager: Option<GlobalHotKeyManager>,
    hotkey_id: Option<u32>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let hotkey = HotKey::new(None, Code::F6);
        let (manager, hotkey_id, status) = match GlobalHotKeyManager::new() {
            Ok(manager) => match manager.register(hotkey) {
                Ok(()) => (
                    Some(manager),
                    Some(hotkey.id()),
                    "Ready — press F6 or Start typing".to_owned(),
                ),
                Err(error) => (
                    Some(manager),
                    None,
                    format!("F6 unavailable ({error}). Use the Start button."),
                ),
            },
            Err(error) => (
                None,
                None,
                format!("Global hotkeys unavailable ({error}). Use the Start button."),
            ),
        };
        Self {
            text: String::new(),
            wpm: 120,
            delay_seconds: 3.0,
            typo_percent: 10.0,
            variation_percent: 15.0,
            active: Arc::new(AtomicBool::new(false)),
            status,
            events: None,
            _hotkey_manager: manager,
            hotkey_id,
        }
    }

    fn toggle(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            self.active.store(false, Ordering::SeqCst);
            self.status = "Stopping…".to_owned();
            return;
        }
        if self.text.trim().is_empty() {
            self.status = "Enter some text first.".to_owned();
            return;
        }
        // Each run owns its cancellation flag, so a quick restart cannot revive an old worker.
        self.active = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = mpsc::channel();
        self.events = Some(receiver);
        self.status = "Get ready to focus the target window…".to_owned();
        typing::spawn(
            self.text.clone(),
            TypingSettings {
                wpm: self.wpm,
                delay_seconds: self.delay_seconds,
                typo_chance: self.typo_percent / 100.0,
                timing_variation: self.variation_percent / 100.0,
            },
            Arc::clone(&self.active),
            sender,
        );
    }

    fn poll(&mut self) {
        if let Some(id) = self.hotkey_id {
            while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                if event.id == id && event.state == HotKeyState::Pressed {
                    self.toggle();
                }
            }
        }
        let pending: Vec<_> = self
            .events
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for event in pending {
            match event {
                TypingEvent::Countdown(n) => {
                    self.status = format!("Focus the target window — typing in {n}…")
                }
                TypingEvent::Typing => self.status = "Typing — press F6 to stop".to_owned(),
                TypingEvent::Finished => self.status = "Finished".to_owned(),
                TypingEvent::Stopped => {
                    self.active.store(false, Ordering::SeqCst);
                    self.status = "Stopped".to_owned();
                }
                TypingEvent::Error(message) => self.status = message,
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Typing Simulator");
            ui.label("Paste text, press Start, then focus the window you want to type into.");
            ui.add_space(8.0);
            ui.add_enabled(
                !self.active.load(Ordering::SeqCst),
                egui::TextEdit::multiline(&mut self.text)
                    .hint_text("Paste or type your text here…")
                    .desired_rows(12)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Typing settings").strong());
                ui.add(egui::Slider::new(&mut self.wpm, 20..=250).text("words per minute"));
                ui.add(egui::Slider::new(&mut self.delay_seconds, 0.0..=10.0).text("start delay (seconds)"));
                ui.add(egui::Slider::new(&mut self.typo_percent, 0.0..=25.0).text("corrected typos (%)"));
                ui.add(egui::Slider::new(&mut self.variation_percent, 0.0..=50.0).text("timing variation (%)"));
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let label = if self.active.load(Ordering::SeqCst) { "Stop typing" } else { "Start typing" };
                if ui.add_sized([130.0, 36.0], egui::Button::new(label)).clicked() {
                    self.toggle();
                }
                ui.label("Global shortcut: F6");
            });
            ui.add_space(8.0);
            ui.label(&self.status);
            ui.small("macOS: grant Accessibility permission. Windows: typing into an elevated app requires running this app as administrator too.");
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }
}
