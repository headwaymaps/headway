//! Live progress for the indexing run, which is thousands of downloads over
//! many minutes and so needs to say more than "working".
//!
//! On a terminal one line is rewritten in place per feed; piped, the same
//! information prints as whole lines every [`LOG_INTERVAL`], since a carriage
//! return isn't a line there and 4000 of them would bury the failures.
//!
//! Failures print immediately either way. A feed that fails to measure has no
//! geometry, so it matches no area and is invisible in the output CSV; the
//! moment it fails is the one place it's cheap to notice.

use crate::measure::{Measurement, Outcome};

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
    ///
    /// Called from every worker, so the whole update is under one lock -
    /// otherwise two threads interleave mid-line.
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
            // Column zero, then clear to end of line: without the clear, a
            // shorter line leaves the tail of the previous one behind.
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

    /// Extrapolates from the feeds done so far. None until there are enough to
    /// extrapolate from: an estimate off the first two looks just as
    /// authoritative as a good one.
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
///
/// Both halves earn their keep: servers answer with HTML or JSON bodies, which
/// the error quotes to make that recognizable - and the quote arrives as one
/// long line, so cutting at the first newline alone bounds nothing.
fn summarize_error(text: &str) -> String {
    const MAX: usize = 100;

    let line = text.lines().next().unwrap_or(text).trim();
    if line.chars().count() <= MAX {
        return line.to_owned();
    }
    // By chars, not bytes: slicing a multi-byte character in half would panic.
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
    fn scales_bytes_to_a_readable_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn durations_read_as_time_not_seconds() {
        assert_eq!(format_duration(Duration::from_secs(9)), "9s");
        assert_eq!(format_duration(Duration::from_secs(75)), "1m15s");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1h02m");
    }

    #[test]
    fn rate_does_not_divide_by_zero() {
        assert_eq!(format_rate(1024, Duration::ZERO), "- MB/s");
        assert_eq!(
            format_rate(10 * 1024 * 1024, Duration::from_secs(10)),
            "1.0 MB/s"
        );
    }

    #[test]
    fn large_counts_are_grouped() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(42), "42");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(4069), "4,069");
        assert_eq!(thousands(1234567), "1,234,567");
    }

    #[test]
    fn a_multiline_error_is_reduced_to_its_first_line() {
        assert_eq!(summarize_error("bad zip\n  at offset 12\n  ..."), "bad zip");
        assert_eq!(summarize_error("plain"), "plain");
    }

    #[test]
    fn a_long_single_line_error_is_truncated() {
        // The real case: a server answers with an HTML page, and the message
        // quotes it. There's no newline to cut at.
        let html = format!("not a zip archive: {}", "<!DOCTYPE html>".repeat(20));
        let summary = summarize_error(&html);
        assert!(summary.chars().count() <= 101, "{summary}");
        assert!(summary.ends_with('…'), "{summary}");
        assert!(summary.starts_with("not a zip archive:"), "{summary}");
    }

    #[test]
    fn truncation_does_not_split_a_multibyte_character() {
        // Slicing by byte offset would panic here rather than truncate.
        let text = "é".repeat(200);
        let summary = summarize_error(&text);
        assert_eq!(summary.chars().count(), 101);
    }

    #[test]
    fn no_eta_until_there_is_enough_to_extrapolate_from() {
        let progress = Progress::new(1000);
        let state = State {
            done: 5,
            failed: 0,
            bytes: 0,
            line_pending: false,
            last_logged: Instant::now(),
        };
        assert!(progress.eta(&state, Duration::from_secs(10)).is_none());
    }

    #[test]
    fn eta_extrapolates_from_the_rate_so_far() {
        let progress = Progress::new(100);
        let state = State {
            done: 50,
            failed: 0,
            bytes: 0,
            line_pending: false,
            last_logged: Instant::now(),
        };
        // Half done in 60s, so about 60s left.
        let eta = progress.eta(&state, Duration::from_secs(60)).unwrap();
        assert_eq!(eta.as_secs(), 60);
    }

    #[test]
    fn no_eta_once_everything_is_done() {
        let progress = Progress::new(10);
        let state = State {
            done: 10,
            failed: 0,
            bytes: 0,
            line_pending: false,
            last_logged: Instant::now(),
        };
        assert!(progress.eta(&state, Duration::from_secs(60)).is_none());
    }

    #[test]
    fn the_progress_line_mentions_failures_only_when_there_are_some() {
        let progress = Progress::new(100);
        let mut state = State {
            done: 30,
            failed: 0,
            bytes: 1024 * 1024,
            line_pending: false,
            last_logged: Instant::now(),
        };
        assert!(!progress.render(&state).contains("failed"));

        state.failed = 3;
        let line = progress.render(&state);
        assert!(line.contains("3 failed"), "{line}");
        assert!(line.contains("30/100 feeds"), "{line}");
    }
}
