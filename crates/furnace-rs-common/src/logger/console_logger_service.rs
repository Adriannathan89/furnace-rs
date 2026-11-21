//! Timestamped standard-error implementation of the logger backend contract.

use std::borrow::Cow;

use chrono::Local;

use super::{LogLevel, LoggerService};

/// Default logger that writes timestamped messages to standard error.
///
/// Messages often carry request-derived text. Before writing, control
/// characters are rendered as visible escapes and continuation lines are
/// indented, so logged input can neither drive the terminal (for example with
/// ANSI escape sequences) nor forge a line that looks like a separate record.
pub struct ConsoleLoggerService;

/// Indentation marking a continuation line as part of the preceding record.
const CONTINUATION_INDENT: &str = "    ";

/// Neutralizes terminal control and record-forging characters in a message.
///
/// Tabs are kept. Line feeds are kept but followed by an indent, so no
/// continuation line can start with a timestamp. Every other control character,
/// including carriage return, escape, and the C1 range, becomes a `\u{..}` escape.
fn neutralize(message: &str) -> Cow<'_, str> {
    if !message
        .chars()
        .any(|character| character.is_control() && character != '\t')
    {
        return Cow::Borrowed(message);
    }

    let mut safe = String::with_capacity(message.len() + CONTINUATION_INDENT.len());

    for character in message.chars() {
        match character {
            '\t' => safe.push('\t'),
            '\n' => {
                safe.push('\n');
                safe.push_str(CONTINUATION_INDENT);
            }
            control if control.is_control() => safe.extend(control.escape_unicode()),
            other => safe.push(other),
        }
    }

    Cow::Owned(safe)
}

impl LoggerService for ConsoleLoggerService {
    /// Writes a timestamped, color-coded line to standard error.
    fn log(&self, level: LogLevel, message: &str) {
        let level_str = match level {
            LogLevel::Trace => "\x1b[34mTRACE\x1b[0m", // Blue
            LogLevel::Debug => "\x1b[36mDEBUG\x1b[0m", // Cyan
            LogLevel::Info => "\x1b[32mINFO\x1b[0m",   // Green
            LogLevel::Warn => "\x1b[33mWARN\x1b[0m",   // Yellow
            LogLevel::Error => "\x1b[31mERROR\x1b[0m", // Red
            LogLevel::Fatal => "\x1b[31mFATAL\x1b[0m", // Red
        };
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        eprintln!("{} - [{}] {}", timestamp, level_str, neutralize(message));
    }
}

#[cfg(test)]
mod tests {
    use super::neutralize;
    use std::borrow::Cow;

    #[test]
    fn ordinary_messages_are_written_unchanged_without_allocating() {
        for message in ["", "user 42 signed in", "tab\tseparated", "密碼🔐 café"] {
            assert!(matches!(neutralize(message), Cow::Borrowed(same) if same == message));
        }
    }

    #[test]
    fn terminal_control_sequences_become_visible_escapes() {
        assert_eq!(
            neutralize("name=\x1b[2J\x1b]0;owned\x07 done"),
            "name=\\u{1b}[2J\\u{1b}]0;owned\\u{7} done"
        );
        assert_eq!(
            neutralize("a\u{9b}31mb\0c\u{7f}"),
            "a\\u{9b}31mb\\u{0}c\\u{7f}"
        );
    }

    #[test]
    fn injected_lines_cannot_start_a_record_at_column_zero() {
        let forged = "login failed\r\n2026-01-01 00:00:00 - [INFO] admin signed in";

        assert_eq!(
            neutralize(forged),
            "login failed\\u{d}\n    2026-01-01 00:00:00 - [INFO] admin signed in"
        );
        assert_eq!(
            neutralize("message\nTrace: frames\nContext: job"),
            "message\n    Trace: frames\n    Context: job"
        );
    }
}
