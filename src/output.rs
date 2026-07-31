//! The agent output contract: `--json` gives raw pretty JSON, everything else
//! gives hand-formatted, column-aligned tables. Write commands echo the
//! affected task reference so a caller can chain without a second read.

use serde::Serialize;

use crate::api::models::Task;

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("Could not serialize the response as JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Pretty-print any serializable value as the machine-readable output.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), OutputError> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// A left-aligned text table with two spaces between columns.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Self {
            headers: headers.iter().map(|header| header.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    /// Add a row. Short rows are padded, extra cells are kept (and widen nothing).
    pub fn row<I, S>(&mut self, cells: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.rows
            .push(cells.into_iter().map(Into::into).collect::<Vec<String>>());
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Print the table, or `empty_message` when there are no rows.
    pub fn print_or(&self, empty_message: &str) {
        if self.rows.is_empty() {
            println!("{empty_message}");
        } else {
            self.print();
        }
    }

    pub fn print(&self) {
        let mut widths: Vec<usize> = self
            .headers
            .iter()
            .map(|header| display_width(header))
            .collect();
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                let width = display_width(cell);
                match widths.get_mut(index) {
                    Some(current) => *current = (*current).max(width),
                    None => widths.push(width),
                }
            }
        }
        print_row(&self.headers, &widths);
        for row in &self.rows {
            print_row(row, &widths);
        }
    }
}

fn print_row(cells: &[String], widths: &[usize]) {
    let mut line = String::new();
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            line.push_str("  ");
        }
        line.push_str(cell);
        // The last column is never padded, so lines carry no trailing spaces.
        if index + 1 < cells.len() {
            let padding = widths.get(index).copied().unwrap_or(0);
            for _ in display_width(cell)..padding {
                line.push(' ');
            }
        }
    }
    println!("{}", line.trim_end());
}

/// Column width in characters; good enough for the Latin text on these boards.
fn display_width(text: &str) -> usize {
    text.chars().count()
}

/// Confirm a write by echoing the affected task's reference and ID.
pub fn print_affected_task(task: &Task) {
    print_affected(&task.id, task.number.as_ref().map(ToString::to_string));
}

/// Same, when only the ID (and possibly a number) is at hand — e.g. after create.
pub fn print_affected(task_id: &str, number: Option<String>) {
    match number {
        Some(number) => println!("{number} ({task_id})"),
        None => println!("{task_id}"),
    }
}

/// Collapse a multi-line value into one table cell, truncated to `max_chars`.
pub fn truncate_cell(text: &str, max_chars: usize) -> String {
    let single_line = text.replace(['\n', '\r'], " ");
    let trimmed = single_line.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    trimmed.chars().take(keep).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_cell_flattens_and_shortens() {
        assert_eq!(truncate_cell("  hello\nworld ", 20), "hello world");
        assert_eq!(truncate_cell("abcdefgh", 4), "abc…");
    }

    #[test]
    fn table_reports_emptiness() {
        let mut table = Table::new(&["NUMBER", "NAME"]);
        assert!(table.is_empty());
        table.row(["E613", "Write report"]);
        assert!(!table.is_empty());
    }
}
