//! The master view used in `--dual` mode.
//!
//! This shows the current slide and a preview of the next one, both using the presentation's theme,
//! and the speaker notes in a panel along the bottom. The panel and labels have a fixed look that
//! does not depend on the theme so that notes are always readable.

use crate::{
    markdown::text_style::{Color, Colors, TextStyle},
    presentation::{Presentation, Slide},
    render::{
        RenderResult, TerminalDrawer,
        engine::{RenderEngine, RenderEngineOptions},
        operation::RenderOperation,
        properties::WindowSize,
    },
    terminal::{
        printer::{TerminalCommand, TerminalIo},
        region::RegionTerminal,
    },
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MIN_COLUMNS: u16 = 40;
const MIN_ROWS: u16 = 14;
const MIN_COLUMNS_WITH_PREVIEW: u16 = 90;
const MIN_PREVIEW_COLUMNS: u16 = 30;
const NOTES_PADDING: u16 = 2;
const NOTES_EMPTY_TEXT: &str = "No speaker notes for this slide.";
const NOTES_OVERFLOW_TEXT: &str = "… more notes than fit, make the terminal taller";

const NOTES_COLORS: Colors = Colors {
    background: Some(Color::Rgb { r: 22, g: 22, b: 30 }),
    foreground: Some(Color::Rgb { r: 236, g: 236, b: 236 }),
};
const NOTES_DIM_COLORS: Colors = Colors {
    background: Some(Color::Rgb { r: 22, g: 22, b: 30 }),
    foreground: Some(Color::Rgb { r: 130, g: 130, b: 150 }),
};
const LABEL_COLORS: Colors = Colors {
    background: Some(Color::Rgb { r: 52, g: 52, b: 72 }),
    foreground: Some(Color::Rgb { r: 255, g: 255, b: 255 }),
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) column: u16,
    pub(crate) row: u16,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

/// Where everything goes in the master terminal.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DualLayout {
    pub(crate) current: Rect,
    pub(crate) notes: Option<Rect>,
    pub(crate) next: Option<Rect>,
}

impl DualLayout {
    pub(crate) fn compute(columns: u16, rows: u16) -> Self {
        if columns < MIN_COLUMNS || rows < MIN_ROWS {
            return Self { current: Rect { column: 0, row: 0, columns, rows }, notes: None, next: None };
        }
        // The notes panel takes the full width and includes a label row.
        let notes_total = (rows * 4 / 10).max(8);
        let main_rows = rows - notes_total;
        let (left_columns, next) = if columns >= MIN_COLUMNS_WITH_PREVIEW {
            let right = (columns * 4 / 10).max(MIN_PREVIEW_COLUMNS);
            let left = columns - right - 1;
            // Keep the terminal's aspect ratio in the preview. Row 0 is taken by the label.
            let preview_rows = ((right as u32 * rows as u32) / columns as u32) as u16;
            let preview_rows = preview_rows.clamp(1, main_rows - 1);
            (left, Some(Rect { column: left + 1, row: 1, columns: right, rows: preview_rows }))
        } else {
            (columns, None)
        };
        Self {
            current: Rect { column: 0, row: 0, columns: left_columns, rows: main_rows },
            // The first row is taken by the label.
            notes: Some(Rect { column: 0, row: main_rows + 1, columns, rows: notes_total - 1 }),
            next,
        }
    }
}

fn region_size(full: &WindowSize, rect: &Rect) -> WindowSize {
    let width = (full.pixels_per_column() * rect.columns as f64) as u16;
    let height = (full.pixels_per_row() * rect.rows as f64) as u16;
    WindowSize { rows: rect.rows, columns: rect.columns, width, height }
}

fn label(text: &str, columns: u16) -> String {
    let mut label = format!(" {text}");
    let used = label.chars().count();
    label.extend(std::iter::repeat_n(' ', (columns as usize).saturating_sub(used)));
    label.chars().take(columns as usize).collect()
}

/// The speaker notes visible in the given slide as plain lines. Empty lines separate paragraphs.
pub(crate) fn notes_lines(slide: &Slide) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for operation in slide.iter_visible_operations() {
        match operation {
            RenderOperation::RenderText { line, .. } => current.push_str(&line.plain_text()),
            RenderOperation::RenderLineBreak => lines.push(std::mem::take(&mut current)),
            _ => (),
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    // Don't let multiple consecutive empty lines waste space.
    lines.dedup_by(|a, b| a.trim().is_empty() && b.trim().is_empty());
    lines
}

/// Wrap a line at word boundaries so it fits in `width` columns.
pub(crate) fn wrap(line: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    if line.trim().is_empty() {
        return vec![String::new()];
    }
    let mut output = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    for word in line.split_whitespace() {
        let mut word = word;
        // Words that are too long to fit in a line on their own get split.
        while word.width() > width {
            if !current.is_empty() {
                output.push(std::mem::take(&mut current));
            }
            let mut taken = 0;
            let mut end = 0;
            for (index, c) in word.char_indices() {
                let char_width = c.width().unwrap_or(0);
                if taken + char_width > width {
                    break;
                }
                taken += char_width;
                end = index + c.len_utf8();
            }
            if end == 0 {
                // Not even a single character fits.
                return output;
            }
            output.push(word[..end].to_string());
            word = &word[end..];
        }
        let word_width = word.width();
        if current.is_empty() {
            current.push_str(word);
            current_width = word_width;
        } else if current_width + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            output.push(std::mem::take(&mut current));
            current.push_str(word);
            current_width = word_width;
        }
    }
    if !current.is_empty() {
        output.push(current);
    }
    output
}

/// Wrap all notes lines so they fit in `width` columns.
pub(crate) fn wrap_notes(lines: &[String], width: usize) -> Vec<String> {
    lines.iter().flat_map(|line| wrap(line, width)).collect()
}

/// Render the master view.
pub(crate) fn render(
    drawer: &mut TerminalDrawer,
    presentation: &Presentation,
    notes: &mut Presentation,
    font_size_fallback: u8,
) -> RenderResult {
    let window = WindowSize::current(font_size_fallback)?;
    let layout = DualLayout::compute(window.columns, window.rows);
    let slide = presentation.current_slide();

    // The notes presentation has the same slides so just follow the main presentation.
    notes.go_to_slide(presentation.current_slide_index());
    notes.jump_chunk(slide.current_chunk_index());

    drawer.terminal.execute(&TerminalCommand::BeginUpdate)?;
    let result = render_panes(drawer, presentation, notes, &window, &layout);
    drawer.terminal.execute(&TerminalCommand::EndUpdate)?;
    drawer.terminal.execute(&TerminalCommand::Flush)?;
    result
}

fn render_panes(
    drawer: &mut TerminalDrawer,
    presentation: &Presentation,
    notes: &Presentation,
    window: &WindowSize,
    layout: &DualLayout,
) -> RenderResult {
    let slide_index = presentation.current_slide_index();
    let options = drawer.render_engine_options();

    // This one clears the whole screen so it has to go first.
    let mut region = RegionTerminal::new(
        &mut drawer.terminal,
        layout.current.column,
        layout.current.row,
        layout.current.columns,
        layout.current.rows,
        true,
    );
    let engine = RenderEngine::new(&mut region, region_size(window, &layout.current), options);
    engine.render(presentation.current_slide().iter_visible_operations())?;

    let slide_count = presentation.iter_slides().count();
    if let Some(rect) = &layout.next {
        if let Some(next_slide) = presentation.iter_slides().nth(slide_index + 1) {
            let mut region =
                RegionTerminal::new(&mut drawer.terminal, rect.column, rect.row, rect.columns, rect.rows, false);
            let engine = RenderEngine::new(&mut region, region_size(window, rect), RenderEngineOptions::default());
            engine.render(next_slide.iter_operations())?;
        }
    }

    // Everything from here on is drawn directly and may be anywhere in the screen.
    drawer.terminal.execute(&TerminalCommand::SetCursorBoundaries { rows: window.rows })?;
    if let Some(rect) = &layout.next {
        let text = if slide_index + 1 < slide_count {
            format!("Next: slide {} of {}", slide_index + 2, slide_count)
        } else {
            "Next: end of presentation".to_string()
        };
        print_line(drawer, rect.column, 0, &label(&text, rect.columns), LABEL_COLORS)?;
    }
    if let Some(rect) = &layout.notes {
        let text = format!("Speaker notes - slide {} of {}", slide_index + 1, slide_count);
        print_line(drawer, 0, rect.row - 1, &label(&text, rect.columns), LABEL_COLORS)?;
        render_notes(drawer, rect, &notes_lines(notes.current_slide()))?;
    }
    Ok(())
}

fn render_notes(drawer: &mut TerminalDrawer, rect: &Rect, notes: &[String]) -> RenderResult {
    let text_columns = rect.columns.saturating_sub(NOTES_PADDING * 2) as usize;
    let mut lines = wrap_notes(notes, text_columns);
    let mut colors = NOTES_COLORS;
    if lines.is_empty() {
        lines.push(NOTES_EMPTY_TEXT.to_string());
        colors = NOTES_DIM_COLORS;
    }
    // The first row is left blank as padding.
    let visible_rows = rect.rows.saturating_sub(1) as usize;
    let overflow = lines.len() > visible_rows;
    if overflow {
        lines.truncate(visible_rows.saturating_sub(1));
    }
    let blank = " ".repeat(rect.columns as usize);
    for row in 0..rect.rows {
        print_line(drawer, rect.column, rect.row + row, &blank, NOTES_COLORS)?;
    }
    for (index, line) in lines.iter().enumerate() {
        let row = rect.row + 1 + index as u16;
        print_line(drawer, rect.column + NOTES_PADDING, row, line, colors)?;
    }
    if overflow {
        let row = rect.row + rect.rows - 1;
        print_line(drawer, rect.column + NOTES_PADDING, row, NOTES_OVERFLOW_TEXT, NOTES_DIM_COLORS)?;
    }
    Ok(())
}

fn print_line(drawer: &mut TerminalDrawer, column: u16, row: u16, text: &str, colors: Colors) -> RenderResult {
    drawer.terminal.execute(&TerminalCommand::MoveTo { column, row })?;
    drawer
        .terminal
        .execute(&TerminalCommand::PrintText { content: text, style: TextStyle::default().colors(colors) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_terminal_has_everything() {
        let layout = DualLayout::compute(200, 50);
        assert_eq!(layout.current, Rect { column: 0, row: 0, columns: 119, rows: 30 });
        // The notes panel spans the entire width.
        assert_eq!(layout.notes, Some(Rect { column: 0, row: 31, columns: 200, rows: 19 }));
        let next = layout.next.expect("no preview");
        assert_eq!(next.column, 120);
        assert_eq!(next.columns, 80);
        assert_eq!(next.rows, 20);
    }

    #[test]
    fn regions_do_not_overlap_or_overflow() {
        for (columns, rows) in [(200, 50), (120, 30), (90, 24), (89, 24), (60, 20), (40, 14), (200, 14)] {
            let layout = DualLayout::compute(columns, rows);
            let mut rects = vec![layout.current];
            rects.extend(layout.notes);
            rects.extend(layout.next);
            for rect in &rects {
                assert!(rect.column + rect.columns <= columns, "{columns}x{rows}: {rect:?}");
                assert!(rect.row + rect.rows <= rows, "{columns}x{rows}: {rect:?}");
            }
            for (i, a) in rects.iter().enumerate() {
                for b in &rects[i + 1..] {
                    let columns_overlap = a.column < b.column + b.columns && b.column < a.column + a.columns;
                    let rows_overlap = a.row < b.row + b.rows && b.row < a.row + a.rows;
                    assert!(!(columns_overlap && rows_overlap), "{columns}x{rows}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn narrow_terminal_drops_preview() {
        let layout = DualLayout::compute(80, 24);
        assert!(layout.next.is_none());
        assert!(layout.notes.is_some());
        assert_eq!(layout.current.columns, 80);
    }

    #[test]
    fn tiny_terminal_only_shows_current() {
        let layout = DualLayout::compute(30, 10);
        assert_eq!(layout.current, Rect { column: 0, row: 0, columns: 30, rows: 10 });
        assert!(layout.notes.is_none());
        assert!(layout.next.is_none());
    }

    #[test]
    fn labels_fit() {
        assert_eq!(label("Notes", 10), " Notes    ");
        assert_eq!(label("Notes", 4).chars().count(), 4);
    }

    #[test]
    fn wrapping() {
        assert_eq!(wrap("the quick brown fox", 10), vec!["the quick", "brown fox"]);
        assert_eq!(wrap("short", 10), vec!["short"]);
        assert_eq!(wrap("", 10), vec![""]);
        assert_eq!(wrap("   ", 10), vec![""]);
    }

    #[test]
    fn wrapping_long_words() {
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("a abcdefgh", 4), vec!["a", "abcd", "efgh"]);
    }

    #[test]
    fn wrapping_wide_characters() {
        // Each of these takes up 2 columns.
        assert_eq!(wrap("你好世界", 4), vec!["你好", "世界"]);
        assert_eq!(wrap("你好世界", 1), Vec::<String>::new());
    }

    #[test]
    fn wrapping_paragraphs() {
        let lines = vec!["one two three".to_string(), String::new(), "four".to_string()];
        assert_eq!(wrap_notes(&lines, 7), vec!["one two", "three", "", "four"]);
    }
}
