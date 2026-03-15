use std::fmt;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::sync::Mutex;
use crate::{dprintln, dprint};

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Green, Yellow, Red, Reset,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Red => "\x1b[31m",
            Color::Reset => "\x1b[0m",
        };
        write!(f, "{}", code)
    }
}

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct ProgressBar {
    pub total: usize,
    pub done: Arc<AtomicUsize>,
    pub start_time: Instant,
    pub last_update: Arc<Mutex<Instant>>,
    pub running: Arc<AtomicBool>,
    pub width: usize,
    pub handle: Mutex<Option<JoinHandle<()>>>,
    pub label: String,
    pub indent: usize,
    pub failed: Arc<AtomicBool>,
    pub finished: Arc<AtomicBool>,
}

impl ProgressBar {
    pub fn new(total: usize, label: impl Into<String>, width: Option<usize>) -> Self {
        Self::with_indent(total, width.unwrap_or(80), label, 4)
    }

    pub fn with_indent(total: usize, width: usize, label: impl Into<String>, indent: usize) -> Self {
        Self {
            total,
            done: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            last_update: Arc::new(Mutex::new(Instant::now())),
            running: Arc::new(AtomicBool::new(true)),
            width,
            handle: Mutex::new(None),
            label: label.into(),
            indent,
            failed: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn error(&self, message: Option<&str>) {
        self.failed.store(true, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);

        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }

        let msg = match message {
            Some(m) => m.to_string(),
            None => format!("{} failed", self.label),
        };

        dprintln!("");
        dprintln!("{}{}{}", Color::Red, "-".repeat(msg.len()), Color::Reset);
        dprintln!("{}✘{} {}", Color::Red, Color::Reset, msg);
    }

    pub fn build_bar_string(
        current_done: usize,
        total: usize,
        width: usize,
        label: &str,
        _elapsed: f64,
    ) -> (String, String) {
        let mb_done = current_done as f64 / 1024.0 / 1024.0;

        if total > 0 {
            let ratio = (current_done as f64 / total as f64).min(1.0);
            let filled_len = (width as f64 * ratio) as usize;

            let bar_str = format!(
                "{}{}{}{}",
                Color::Green,
                "#".repeat(filled_len),
                Color::Reset,
                "-".repeat(width.saturating_sub(filled_len))
            );

            (bar_str, format!("{:3.1}%", ratio * 100.0))
        } else {
            (label.to_string(), format!("{:.1} MiB", mb_done))
        }
    }

    pub fn render(indent: usize, color: Color, frame: &str, bar: &str, status: &str, speed: f64) {
        let spaces = " ".repeat(indent);
        dprint!(
            "\r{}{}{}{} [{}] {} ({:.2} MiB/s)\x1b[0m",
            color,
            spaces,
            frame,
            Color::Reset,
            bar,
            status,
            speed
        );
        let _ = io::stdout().flush();
    }

    pub fn update(&self, amount: usize) {
        self.done.fetch_add(amount, Ordering::Relaxed);
        if let Ok(mut last) = self.last_update.lock() {
            *last = Instant::now();
        }
    }

    pub fn start(&self) {
        let done = Arc::clone(&self.done);
        let running = Arc::clone(&self.running);
        let last_update = Arc::clone(&self.last_update);
        let start_time = self.start_time;
        let width = self.width;
        let total = self.total;
        let label = self.label.clone();
        let indent = self.indent;

        let thread_handle = thread::spawn(move || {
            let mut frame_idx = 0;
            let mut last_speed = 0.0;
            let mut last_done = 0;
            let mut last_window_ts = Instant::now();

            while running.load(Ordering::SeqCst) {
                let now = Instant::now();

                let last_ts = {
                    let guard = last_update.lock().unwrap();
                    *guard
                };

                let delta = now.duration_since(last_ts).as_secs_f64();

                let color = if delta < 0.2 {
                    Color::Green
                } else if delta < 1.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let current_done = done.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();

                let window_elapsed = now.duration_since(last_window_ts).as_secs_f64();
                if window_elapsed >= 0.25 {
                    let bytes_since = current_done.saturating_sub(last_done);
                    last_speed = (bytes_since as f64 / 1024.0 / 1024.0) / window_elapsed;
                    last_done = current_done;
                    last_window_ts = now;
                }
                
                let speed = last_speed;

                let (bar, status) =
                    ProgressBar::build_bar_string(current_done, total, width, &label, elapsed);

                let frame = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
                ProgressBar::render(indent, color, frame, &bar, &status, speed);

                frame_idx += 1;

                let sleep_val = (delta.clamp(0.05, 0.2) * 1000.0) as u64;
                thread::sleep(Duration::from_millis(sleep_val));
            }
        });

        *self.handle.lock().unwrap() = Some(thread_handle);
    }

    pub fn finish(&self, overwrite: bool) {
        if self.finished.swap(true, Ordering::SeqCst) {
            return;
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }

        if self.failed.load(Ordering::SeqCst) {
            return;
        }

        let current_done = self.done.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();

        let mb_done = current_done as f64 / 1024.0 / 1024.0;
        let speed = if elapsed > 0.0 { mb_done / elapsed } else { 0.0 };

        let (bar, status) =
            Self::build_bar_string(current_done, self.total, self.width, &self.label, elapsed);

        let spaces = " ".repeat(self.indent);

        dprint!(
            "\r{}{}  [{}] {} ({:.2} MiB/s)\x1b[0m",
            Color::Green,
            spaces,
            bar,
            status,
            speed
        );

        let _ = io::stdout().flush();

        if overwrite {
            dprint!("\r\x1b[K");
        } else {
            dprintln!("");
        }

        dprintln!("{}✔{} Task complete!", Color::Green, Color::Reset);
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.finish(false);
    }
}