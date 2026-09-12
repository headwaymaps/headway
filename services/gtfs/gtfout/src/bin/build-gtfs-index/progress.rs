//! Live progress for the indexing run, which is thousands of downloads over
//! many minutes and so needs to say more than "working".

use gtfout::measure::{Measurement, Outcome};

use std::io::{IsTerminal, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How often to print when output isn't a terminal.
const LOG_INTERVAL: Duration = Duration::from_secs(15);

/// Feeds this size are worth naming: a single one can account for minutes of a
/// run, and otherwise the progress line just appears to stall.
const LARGE_FEED: u64 = 100 * 1024 * 1024;

pub struct Progress {
    total: usize,
    start: Instant,
    interactive: bool,
    state: Mutex<State>,
}

struct State {
    done: usize,
    failed: usize,
    bytes: u64,
    /// An in-place line is on screen and must be cleared before anything else.
    line_pending: bool,
    last_logged: Instant,
}

impl Progress {
    pub fn new(total: usize) -> Self {
        let now = Instant::now();
        Self {
            total,
            start: now,
            interactive: std::io::stderr().is_terminal(),
            state: Mutex::new(State {
                done: 0,
                failed: 0,
                bytes: 0,
                line_pending: false,
                last_logged: now,
            }),
        }
    }

    /// Records one finished feed and updates the display.
    pub fn record(&self, feed_id: &str, outcome: &Outcome) {
        let mut state = self.state.lock().unwrap();

        state.done += 1;
        state.bytes += outcome.bytes;
        if let Measurement::Failed { error } = &outcome.measurement {
            state.failed += 1;
            let error = summarize_error(error);
            self.clear_line(&mut state);
            eprintln!("  failed {feed_id}: {error}");
        } else if outcome.bytes >= LARGE_FEED {
            self.clear_line(&mut state);
            eprintln!("  {feed_id} is {} on its own", format_bytes(outcome.bytes));
        }

        if self.interactive {
            let line = self.render(&state);
            let mut stderr = std::io::stderr();
            let _ = write!(stderr, "\r\x1b[K{line}");
            let _ = stderr.flush();
            state.line_pending = true;
        } else if state.last_logged.elapsed() >= LOG_INTERVAL || state.done == self.total {
            let line = self.render(&state);
            eprintln!("  {line}");
            state.last_logged = Instant::now();
        }
    }

    /// Final tally.
    pub fn finish(&self) {
        let mut state = self.state.lock().unwrap();
        self.clear_line(&mut state);

        let elapsed = self.start.elapsed();
        let measured = state.done - state.failed;
        let mut summary = format!(
            "measured {}/{} feeds in {} · {} at {}",
            thousands(measured as u64),
            thousands(self.total as u64),
            format_duration(elapsed),
            format_bytes(state.bytes),
            format_rate(state.bytes, elapsed),
        );
        if state.failed > 0 {
            summary += &format!(" · {} failed", thousands(state.failed as u64));
        }
        eprintln!("{summary}");
    }

    fn clear_line(&self, state: &mut State) {
        if state.line_pending {
            let _ = write!(std::io::stderr(), "\r\x1b[K");
            state.line_pending = false;
        }
    }

    fn render(&self, state: &State) -> String {
        let elapsed = self.start.elapsed();
        let mut line = format!(
            "{}/{} feeds",
            thousands(state.done as u64),
            thousands(self.total as u64)
        );
        if state.failed > 0 {
            line += &format!(" · {} failed", state.failed);
        }
        line += &format!(
            " · {} · {} · {} elapsed",
            format_bytes(state.bytes),
            format_rate(state.bytes, elapsed),
            format_duration(elapsed),
        );
        if let Some(remaining) = self.eta(state, elapsed) {
            line += &format!(" · ~{} left", format_duration(remaining));
        }
        line
    }

    fn eta(&self, state: &State, elapsed: Duration) -> Option<Duration> {
        const MIN_SAMPLE: usize = 20;
        if state.done < MIN_SAMPLE || state.done >= self.total {
            return None;
        }
        let per_feed = elapsed.as_secs_f64() / state.done as f64;
        let remaining = (self.total - state.done) as f64 * per_feed;
        Some(Duration::from_secs_f64(remaining))
    }
}

/// Trims an error down to one short line. The full text stays in the index.
fn summarize_error(text: &str) -> String {
    const MAX: usize = 100;

    let line = text.lines().next().unwrap_or(text).trim();
    if line.chars().count() <= MAX {
        return line.to_owned();
    }
    let truncated: String = line.chars().take(MAX).collect();
    format!("{}…", truncated.trim_end())
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_rate(bytes: u64, elapsed: Duration) -> String {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return "- MB/s".to_owned();
    }
    format!("{:.1} MB/s", bytes as f64 / seconds / (1024.0 * 1024.0))
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, (total % 3600) / 60, total % 60);
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_does_not_divide_by_zero() {
        assert_eq!(format_rate(1024, Duration::ZERO), "- MB/s");
        assert_eq!(
            format_rate(10 * 1024 * 1024, Duration::from_secs(10)),
            "1.0 MB/s"
        );
    }

    #[test]
    fn truncation_does_not_split_a_multibyte_character() {
        let text = "é".repeat(200);
        let summary = summarize_error(&text);
        assert_eq!(summary.chars().count(), 101);
    }

    fn state(done: usize) -> State {
        State {
            done,
            failed: 0,
            bytes: 0,
            line_pending: false,
            last_logged: Instant::now(),
        }
    }

    #[test]
    fn eta_extrapolates_from_the_rate_so_far() {
        let eta = Progress::new(100)
            .eta(&state(50), Duration::from_secs(60))
            .unwrap();
        assert_eq!(eta.as_secs(), 60);

        assert!(Progress::new(1000)
            .eta(&state(5), Duration::from_secs(10))
            .is_none());

        assert!(Progress::new(10)
            .eta(&state(10), Duration::from_secs(60))
            .is_none());
    }
}
