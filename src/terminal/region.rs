use super::printer::{TerminalCommand, TerminalError, TerminalIo};
use crate::markdown::text_style::{Colors, TextStyle};

/// A terminal that confines everything drawn on it to a rectangle of an underlying terminal.
///
/// All coordinates are relative to the top left corner of the region.
pub(crate) struct RegionTerminal<'a, T: TerminalIo> {
    inner: &'a mut T,
    column: u16,
    row: u16,
    columns: u16,
    rows: u16,
    colors: Colors,
    // Whether clearing the screen clears the entire underlying terminal or only this region.
    clear_everything: bool,
}

impl<'a, T: TerminalIo> RegionTerminal<'a, T> {
    pub(crate) fn new(
        inner: &'a mut T,
        column: u16,
        row: u16,
        columns: u16,
        rows: u16,
        clear_everything: bool,
    ) -> Self {
        Self { inner, column, row, columns, rows, colors: Default::default(), clear_everything }
    }

    #[cfg(test)]
    fn colors(&self) -> Colors {
        self.colors
    }

    fn clear_region(&mut self) -> Result<(), TerminalError> {
        let blank = " ".repeat(self.columns as usize);
        let style = TextStyle::default().colors(self.colors);
        for row in 0..self.rows {
            self.inner.execute(&TerminalCommand::MoveTo { column: self.column, row: self.row + row })?;
            self.inner.execute(&TerminalCommand::PrintText { content: &blank, style })?;
        }
        self.inner.execute(&TerminalCommand::MoveTo { column: self.column, row: self.row })
    }
}

impl<T: TerminalIo> TerminalIo for RegionTerminal<'_, T> {
    fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
        use TerminalCommand::*;
        match command {
            // The owner of the underlying terminal is in charge of these.
            BeginUpdate | EndUpdate | Flush => Ok(()),
            MoveTo { column, row } => self.inner.execute(&MoveTo { column: column + self.column, row: row + self.row }),
            MoveToRow(row) => self.inner.execute(&MoveToRow(row + self.row)),
            MoveToColumn(column) => self.inner.execute(&MoveToColumn(column + self.column)),
            MoveToNextLine => {
                self.inner.execute(&MoveToNextLine)?;
                if self.column > 0 { self.inner.execute(&MoveToColumn(self.column)) } else { Ok(()) }
            }
            SetColors(colors) => {
                self.colors = *colors;
                self.inner.execute(command)
            }
            SetBackgroundColor(color) => {
                self.colors.background = Some(*color);
                self.inner.execute(command)
            }
            ClearScreen if self.clear_everything => self.inner.execute(command),
            ClearScreen => self.clear_region(),
            SetCursorBoundaries { rows } => self.inner.execute(&SetCursorBoundaries { rows: rows + self.row }),
            MoveDown(_) | MoveRight(_) | MoveLeft(_) | PrintText { .. } | PrintImage { .. } => {
                self.inner.execute(command)
            }
        }
    }

    fn cursor_row(&self) -> u16 {
        self.inner.cursor_row().saturating_sub(self.row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::text_style::Color;

    #[derive(Default)]
    struct Recorder {
        commands: Vec<String>,
        row: u16,
    }

    impl TerminalIo for Recorder {
        fn execute(&mut self, command: &TerminalCommand<'_>) -> Result<(), TerminalError> {
            if let TerminalCommand::MoveTo { row, .. } = command {
                self.row = *row;
            }
            self.commands.push(format!("{command:?}"));
            Ok(())
        }

        fn cursor_row(&self) -> u16 {
            self.row
        }
    }

    #[test]
    fn coordinates_are_translated() {
        let mut recorder = Recorder::default();
        let mut region = RegionTerminal::new(&mut recorder, 10, 5, 20, 8, false);
        region.execute(&TerminalCommand::MoveTo { column: 1, row: 2 }).unwrap();
        region.execute(&TerminalCommand::MoveToColumn(3)).unwrap();
        region.execute(&TerminalCommand::MoveToRow(4)).unwrap();
        region.execute(&TerminalCommand::SetCursorBoundaries { rows: 8 }).unwrap();
        assert_eq!(region.cursor_row(), 2);
        assert_eq!(
            recorder.commands,
            vec![
                "MoveTo { column: 11, row: 7 }",
                "MoveToColumn(13)",
                "MoveToRow(9)",
                "SetCursorBoundaries { rows: 13 }",
            ]
        );
    }

    #[test]
    fn next_line_returns_to_region_column() {
        let mut recorder = Recorder::default();
        let mut region = RegionTerminal::new(&mut recorder, 10, 0, 20, 8, false);
        region.execute(&TerminalCommand::MoveToNextLine).unwrap();
        assert_eq!(recorder.commands, vec!["MoveToNextLine", "MoveToColumn(10)"]);
    }

    #[test]
    fn update_and_flush_are_suppressed() {
        let mut recorder = Recorder::default();
        let mut region = RegionTerminal::new(&mut recorder, 0, 0, 20, 8, true);
        region.execute(&TerminalCommand::BeginUpdate).unwrap();
        region.execute(&TerminalCommand::EndUpdate).unwrap();
        region.execute(&TerminalCommand::Flush).unwrap();
        assert!(recorder.commands.is_empty());
    }

    #[test]
    fn clear_only_affects_region() {
        let mut recorder = Recorder::default();
        let mut region = RegionTerminal::new(&mut recorder, 4, 2, 6, 3, false);
        let colors = Colors { background: Some(Color::new(1, 2, 3)), foreground: None };
        region.execute(&TerminalCommand::SetColors(colors)).unwrap();
        region.execute(&TerminalCommand::ClearScreen).unwrap();
        assert_eq!(region.colors(), colors);
        assert!(!recorder.commands.iter().any(|c| c == "ClearScreen"));
        let moves: Vec<_> = recorder.commands.iter().filter(|c| c.starts_with("MoveTo")).collect();
        assert_eq!(moves.len(), 4);
        assert_eq!(moves[0], "MoveTo { column: 4, row: 2 }");
        assert_eq!(moves[2], "MoveTo { column: 4, row: 4 }");
        let prints = recorder.commands.iter().filter(|c| c.starts_with("PrintText")).count();
        assert_eq!(prints, 3);
    }

    #[test]
    fn clear_everything() {
        let mut recorder = Recorder::default();
        let mut region = RegionTerminal::new(&mut recorder, 0, 0, 6, 3, true);
        region.execute(&TerminalCommand::ClearScreen).unwrap();
        assert_eq!(recorder.commands, vec!["ClearScreen"]);
    }
}
