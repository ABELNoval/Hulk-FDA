#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn new(
        file: String,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            file,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        // Asume que ambos spans están en el mismo archivo
        let start_line = self.start_line.min(other.start_line);
        let start_column = if self.start_line < other.start_line {
            self.start_column
        } else if self.start_line > other.start_line {
            other.start_column
        } else {
            self.start_column.min(other.start_column)
        };

        let end_line = self.end_line.max(other.end_line);
        let end_column = if self.end_line > other.end_line {
            self.end_column
        } else if self.end_line < other.end_line {
            other.end_column
        } else {
            self.end_column.max(other.end_column)
        };

        Span::new(
            self.file.clone(),
            start_line,
            start_column,
            end_line,
            end_column,
        )
    }
}
