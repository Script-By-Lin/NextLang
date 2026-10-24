use std::fmt;

/// Represents a source byte offset span `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn empty(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Represents a source file loaded in memory.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub content: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let name = name.into();
        let content = content.into();
        let mut line_starts = vec![0];
        for (i, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            name,
            content,
            line_starts,
        }
    }

    /// Converts a byte offset to (line, col) (1-indexed).
    pub fn location(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.content.len());
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let col = offset - line_start + 1;
        (line_idx + 1, col)
    }

    /// Returns the text content of a line (1-indexed, without trailing `\r` or `\n`).
    pub fn line_text(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[line - 1];
        let end = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            self.content.len()
        };
        let slice = &self.content[start..end];
        Some(slice.trim_end_matches(['\r', '\n']))
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

impl DiagnosticLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
            DiagnosticLevel::Help => "help",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: Option<String>,
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            code: None,
            message: message.into(),
            span,
            help: None,
            notes: Vec::new(),
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            code: None,
            message: message.into(),
            span,
            help: None,
            notes: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Renders this diagnostic to a formatted string using the provided source file.
    pub fn render(&self, file: &SourceFile) -> String {
        let (line, col) = file.location(self.span.start);
        let mut out = String::new();

        // Header: e.g. "error[NX-TYPE-001]: incompatible assignment" or "error: expected ';'"
        if let Some(ref code) = self.code {
            out.push_str(&format!(
                "{}[{}]: {}\n",
                self.level.as_str(),
                code,
                self.message
            ));
        } else {
            out.push_str(&format!("{}: {}\n", self.level.as_str(), self.message));
        }

        // Location: e.g. " --> example.nx:4:17"
        out.push_str(&format!(" --> {}:{}:{}\n", file.name, line, col));

        let line_num_str = line.to_string();
        let pad_len = line_num_str.len();
        let pad = " ".repeat(pad_len);

        out.push_str(&format!("{} |\n", pad));

        if let Some(source_line) = file.line_text(line) {
            out.push_str(&format!("{} | {}\n", line_num_str, source_line));

            // Underline carets
            let underline_start = col.saturating_sub(1);
            let span_len = if self.span.end > self.span.start {
                self.span.end - self.span.start
            } else {
                1
            };
            let remaining_on_line = source_line.len().saturating_sub(underline_start);
            let underline_len = if remaining_on_line == 0 {
                1
            } else {
                span_len.min(remaining_on_line).max(1)
            };

            let spaces = " ".repeat(underline_start);
            let carets = "^".repeat(underline_len);
            out.push_str(&format!("{} | {}{}\n", pad, spaces, carets));
        } else {
            out.push_str(&format!("{} | <end of file>\n", pad));
            out.push_str(&format!("{} | ^\n", pad));
        }

        out.push_str(&format!("{} |\n", pad));

        // Help text
        if let Some(ref help) = self.help {
            out.push_str(&format!("{} = help: {}\n", pad, help));
        }

        // Notes
        for note in &self.notes {
            out.push_str(&format!("{} = note: {}\n", pad, note));
        }

        out
    }
}

/// Accumulator for diagnostics during compiler passes.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn error(&mut self, span: Span, message: impl Into<String>) {
        self.add(Diagnostic::error(span, message));
    }

    pub fn error_code(&mut self, code: impl Into<String>, span: Span, message: impl Into<String>) {
        self.add(Diagnostic::error(span, message).with_code(code));
    }

    pub fn error_with_help(
        &mut self,
        span: Span,
        message: impl Into<String>,
        help: impl Into<String>,
    ) {
        self.add(Diagnostic::error(span, message).with_help(help));
    }

    pub fn error_code_with_help(
        &mut self,
        code: impl Into<String>,
        span: Span,
        message: impl Into<String>,
        help: impl Into<String>,
    ) {
        self.add(
            Diagnostic::error(span, message)
                .with_code(code)
                .with_help(help),
        );
    }

    pub fn warning(&mut self, span: Span, message: impl Into<String>) {
        self.add(Diagnostic::warning(span, message));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn render_all(&self, file: &SourceFile) -> String {
        let mut result = String::new();
        for diag in &self.diagnostics {
            result.push_str(&diag.render(file));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_and_render() {
        let src = SourceFile::new("example.nx", "vout(\"hello\");\nvar age:<INT32> = 20\n");
        let diag = Diagnostic::error(Span::new(31, 33), "expected ';'")
            .with_help("statements must end with ';'");
        let rendered = diag.render(&src);
        assert!(rendered.contains("error: expected ';'"));
        assert!(rendered.contains("--> example.nx:2:17"));
        assert!(rendered.contains("var age:<INT32> = 20"));
        assert!(rendered.contains("= help: statements must end with ';'"));
    }
}
