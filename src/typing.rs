use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub struct TypingSettings {
    pub wpm: u32,
    pub delay_seconds: f32,
    pub typo_chance: f32,
    pub timing_variation: f32,
}

pub enum TypingEvent {
    Countdown(u32),
    Typing,
    Finished,
    Stopped,
    Error(String),
}

pub fn spawn(
    text: String,
    settings: TypingSettings,
    active: Arc<AtomicBool>,
    events: Sender<TypingEvent>,
) {
    thread::spawn(move || {
        let start = Instant::now();
        let delay = Duration::from_secs_f32(settings.delay_seconds);
        let mut last_count = u32::MAX;
        while start.elapsed() < delay {
            if !active.load(Ordering::SeqCst) {
                let _ = events.send(TypingEvent::Stopped);
                return;
            }
            let remaining = (delay - start.elapsed()).as_secs_f32().ceil() as u32;
            if remaining != last_count {
                let _ = events.send(TypingEvent::Countdown(remaining));
                last_count = remaining;
            }
            thread::sleep(Duration::from_millis(20));
        }
        if !active.load(Ordering::SeqCst) {
            let _ = events.send(TypingEvent::Stopped);
            return;
        }

        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(enigo) => enigo,
            Err(error) => {
                fail(&active, &events, error.to_string());
                return;
            }
        };
        let _ = events.send(TypingEvent::Typing);
        let mut rng = rand::rng();

        for character in text.replace("\r\n", "\n").chars() {
            if !active.load(Ordering::SeqCst) {
                let _ = events.send(TypingEvent::Stopped);
                return;
            }
            if character.is_ascii_alphabetic() && rng.random::<f32>() < settings.typo_chance {
                let wrong = rng.random_range(b'a'..=b'z') as char;
                if let Err(error) = enigo.text(&wrong.to_string()) {
                    fail(&active, &events, error.to_string());
                    return;
                }
                if !wait(Duration::from_millis(80), &active) {
                    let _ = events.send(TypingEvent::Stopped);
                    return;
                }
                if let Err(error) = enigo.key(Key::Backspace, Direction::Click) {
                    fail(&active, &events, error.to_string());
                    return;
                }
                if !wait(Duration::from_millis(80), &active) {
                    let _ = events.send(TypingEvent::Stopped);
                    return;
                }
            }
            let result = match character {
                '\n' | '\r' => enigo.key(Key::Return, Direction::Click),
                '\t' => enigo.key(Key::Tab, Direction::Click),
                _ => enigo.text(&character.to_string()),
            };
            if let Err(error) = result {
                fail(&active, &events, error.to_string());
                return;
            }
            // Standard WPM counts five characters as a word.
            let base = 60.0 / (settings.wpm.max(1) as f32 * 5.0);
            let variation =
                rng.random_range(-settings.timing_variation..=settings.timing_variation);
            if !wait(
                Duration::from_secs_f32(base * (1.0 + variation).max(0.05)),
                &active,
            ) {
                let _ = events.send(TypingEvent::Stopped);
                return;
            }
        }
        active.store(false, Ordering::SeqCst);
        let _ = events.send(TypingEvent::Finished);
    });
}

fn wait(duration: Duration, active: &AtomicBool) -> bool {
    let start = Instant::now();
    while start.elapsed() < duration {
        if !active.load(Ordering::SeqCst) {
            return false;
        }
        thread::sleep((duration - start.elapsed()).min(Duration::from_millis(10)));
    }
    active.load(Ordering::SeqCst)
}

fn fail(active: &AtomicBool, events: &Sender<TypingEvent>, error: String) {
    active.store(false, Ordering::SeqCst);
    let _ = events.send(TypingEvent::Error(format!(
        "Keyboard input failed. Check accessibility permissions: {error}"
    )));
}
