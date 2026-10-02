//! Structured diagnostics and framework errors emitted by furnace-rs.

use std::fmt;

/// A stable identifier for a furnace-rs diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    /// Creates a diagnostic code from a static string.
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Returns the string representation of this diagnostic code.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Duplicate provider registration.
pub const FURNACE001: DiagnosticCode = DiagnosticCode::new("FURNACE001");

/// Ambiguous provider binding.
pub const FURNACE002: DiagnosticCode = DiagnosticCode::new("FURNACE002");

/// Missing provider registration.
pub const FURNACE003: DiagnosticCode = DiagnosticCode::new("FURNACE003");

/// Provider registry type mismatch.
pub const FURNACE004: DiagnosticCode = DiagnosticCode::new("FURNACE004");

/// Dependency cycle.
pub const FURNACE005: DiagnosticCode = DiagnosticCode::new("FURNACE005");

/// Provider construction failure.
pub const FURNACE006: DiagnosticCode = DiagnosticCode::new("FURNACE006");

/// Auto-configuration failure.
pub const FURNACE007: DiagnosticCode = DiagnosticCode::new("FURNACE007");

/// Invalid rooted module graph.
pub const FURNACE008: DiagnosticCode = DiagnosticCode::new("FURNACE008");

/// Inaccessible provider across a module boundary.
pub const FURNACE009: DiagnosticCode = DiagnosticCode::new("FURNACE009");

/// Invalid lifecycle state transition.
pub const FURNACE010: DiagnosticCode = DiagnosticCode::new("FURNACE010");

/// Lifecycle hook failure.
pub const FURNACE011: DiagnosticCode = DiagnosticCode::new("FURNACE011");

/// Configuration source failure.
pub const FURNACE020: DiagnosticCode = DiagnosticCode::new("FURNACE020");

/// Conflicting integration route declarations.
pub const FURNACE030: DiagnosticCode = DiagnosticCode::new("FURNACE030");

/// The source position associated with a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceLocation {
    /// The source file path.
    pub file: &'static str,
    /// The one-based source line.
    pub line: u32,
    /// The one-based source column.
    pub column: u32,
}

impl SourceLocation {
    /// Creates a source location from a file, line, and column.
    pub const fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }
}

/// A structured diagnostic with optional source context and suggestions.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    title: String,
    message: String,
    subject: Option<String>,
    location: Option<SourceLocation>,
    suggestions: Vec<String>,
}

impl Diagnostic {
    /// Creates a diagnostic with a code, short title, and detailed message.
    pub fn new(code: DiagnosticCode, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            title: title.into(),
            message: message.into(),
            subject: None,
            location: None,
            suggestions: Vec::new(),
        }
    }

    /// Adds the subject associated with this diagnostic.
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Adds the source location associated with this diagnostic.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Adds a remediation suggestion to this diagnostic.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Returns this diagnostic's stable code.
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Returns this diagnostic's short title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns this diagnostic's detailed message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the subject associated with this diagnostic, when present.
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    /// Returns the source location associated with this diagnostic, when present.
    pub const fn location(&self) -> Option<SourceLocation> {
        self.location
    }

    /// Returns remediation suggestions in insertion order.
    pub fn suggestions(&self) -> &[String] {
        &self.suggestions
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "error[{}]: {}", self.code, self.title)?;
        if let Some(location) = self.location {
            write!(
                formatter,
                "\n  --> {}:{}:{}",
                location.file, location.line, location.column
            )?;
        }
        if let Some(subject) = &self.subject {
            write!(formatter, "\n  = subject: {subject}")?;
        }
        write!(formatter, "\n  = {}", self.message)?;
        for suggestion in &self.suggestions {
            write!(formatter, "\n  help: {suggestion}")?;
        }
        Ok(())
    }
}

/// A framework error containing a structured diagnostic and optional cause.
///
/// Ordinary formatting renders diagnostics without exposing the retained cause.
/// Diagnostic authors must keep their messages safe; access to an underlying
/// cause requires an explicit call to [`std::error::Error::source`].
pub struct Error {
    diagnostics: Vec<Diagnostic>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Error")
            .field("diagnostics", &self.diagnostics)
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

impl Error {
    /// Creates an error from a structured diagnostic.
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
            source: None,
        }
    }

    /// Creates an error from a primary diagnostic and ordered related diagnostics.
    pub fn from_diagnostics(
        primary: Diagnostic,
        related: impl IntoIterator<Item = Diagnostic>,
    ) -> Self {
        let mut diagnostics = vec![primary];
        diagnostics.extend(related);
        Self {
            diagnostics,
            source: None,
        }
    }

    pub(crate) fn with_related_diagnostics(
        mut self,
        related: impl IntoIterator<Item = Diagnostic>,
    ) -> Self {
        self.diagnostics.extend(related);
        self
    }

    /// Creates an error from a structured diagnostic and an underlying cause.
    pub fn with_source<E>(diagnostic: Diagnostic, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            diagnostics: vec![diagnostic],
            source: Some(Box::new(source)),
        }
    }

    /// Returns this error's stable diagnostic code.
    pub fn code(&self) -> DiagnosticCode {
        self.diagnostics[0].code()
    }

    /// Returns the structured diagnostic carried by this error.
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostics[0]
    }

    /// return every diagnostic in deterministic report order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n\n")?;
            }
            diagnostic.fmt(formatter)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

/// The result type used by furnace-rs framework APIs.
pub type Result<T> = std::result::Result<T, Error>;
