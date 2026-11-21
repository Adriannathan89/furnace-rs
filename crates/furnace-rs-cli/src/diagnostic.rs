//! Stable CLI diagnostic codes, structured details, and source-aware errors.

use std::{error::Error, fmt};

use crate::command::ParseError;

pub(crate) const FURNACE200: &str = "FURNACE200";

pub(crate) const FURNACE201: &str = "FURNACE201";

pub(crate) const FURNACE202: &str = "FURNACE202";

pub(crate) const FURNACE204: &str = "FURNACE204";

pub(crate) const FURNACE220: &str = "FURNACE220";

pub(crate) const FURNACE230: &str = "FURNACE230";

#[derive(Default)]
struct CliErrorDetails {
    subject: Option<String>,
    suggestions: Vec<String>,
}

pub(crate) struct CliError {
    code: &'static str,
    title: &'static str,
    message: String,
    details: Box<CliErrorDetails>,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl CliError {
    /// Creates a diagnostic with a stable code, title, safe message, and empty details.
    pub(crate) fn new(code: &'static str, title: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            title,
            message: message.into(),
            details: Box::default(),
            source: None,
        }
    }

    /// Adds a subject to the structured diagnostic.
    pub(crate) fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.details.subject = Some(subject.into());

        self
    }

    /// Appends one remediation suggestion in output order.
    pub(crate) fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.details.suggestions.push(suggestion.into());

        self
    }

    /// Retains an operational cause for deliberate source-chain inspection.
    pub(crate) fn with_source<E>(mut self, source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));

        self
    }

    /// Builds the stable, source-redacting diagnostic for scaffold operations.
    pub(crate) fn scaffolding<E>(message: &'static str, source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::new(FURNACE230, "Project scaffolding failed", message).with_source(source)
    }

    /// Returns the stable diagnostic code.
    pub(crate) const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the short stable diagnostic title.
    pub(crate) const fn title(&self) -> &'static str {
        self.title
    }

    /// Returns the safe diagnostic message.
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// Returns the related subject when one is available.
    pub(crate) fn subject(&self) -> Option<&str> {
        self.details.subject.as_deref()
    }

    /// Returns the ordered remediation suggestions.
    pub(crate) fn suggestions(&self) -> &[String] {
        &self.details.suggestions
    }

    /// Builds the structured diagnostic reserved for CLI grammar failures.
    pub(crate) fn syntax(error: &ParseError) -> Self {
        Self::new(FURNACE204, "CLI syntax error", error.to_string())
    }
}

impl fmt::Display for CliError {
    /// Formats a CLI diagnostic for human display or redacted debug output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "error[{}]: {}", self.code, self.title)?;

        if let Some(subject) = &self.details.subject {
            write!(formatter, "\n  = subject: {subject}")?;
        }

        write!(formatter, "\n  = {}", self.message)?;

        for suggestion in &self.details.suggestions {
            write!(formatter, "\n  help: {suggestion}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for CliError {
    /// Formats a CLI diagnostic for human display or redacted debug output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = if self.source.is_some() {
            "[REDACTED]"
        } else {
            self.message.as_str()
        };
        let source = if self.source.is_some() {
            "[REDACTED]"
        } else {
            "None"
        };

        formatter
            .debug_struct("CliError")
            .field("code", &self.code)
            .field("title", &self.title)
            .field("message", &message)
            .field("subject", &self.details.subject)
            .field("suggestions", &self.details.suggestions)
            .field("source", &source)
            .finish()
    }
}

impl Error for CliError {
    /// Returns the retained operational cause through the standard error chain.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::command::ParseError;

    use super::{CliError, FURNACE200, FURNACE201, FURNACE204};

    #[test]
    fn renders_a_stable_human_diagnostic() {
        let error = CliError::new(
            FURNACE200,
            "Cargo application target is ambiguous",
            "more than one package can be selected",
        )
        .with_subject("workspace")
        .with_suggestion("pass --package <package>");

        assert_eq!(
            error.to_string(),
            "error[FURNACE200]: Cargo application target is ambiguous\n  = subject: workspace\n  = more than one package can be selected\n  help: pass --package <package>"
        );
    }

    #[test]
    fn redacts_a_sourced_diagnostic_from_debug_output() {
        let error = CliError::new(FURNACE201, "Cargo metadata failed", "/absolute/secret")
            .with_source(io::Error::other("/absolute/registry/path"));

        let debug = format!("{error:?}");
        assert!(debug.contains(FURNACE201));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("/absolute"));
    }

    #[test]
    fn reserves_furnace204_for_cli_syntax() {
        let error = CliError::syntax(&ParseError::MissingValue("--format"));

        assert_eq!(error.code(), FURNACE204);
        assert_eq!(
            error.to_string(),
            "error[FURNACE204]: CLI syntax error\n  = missing value for --format"
        );
    }
}
