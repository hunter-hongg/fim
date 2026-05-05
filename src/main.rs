use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

/// A simple file inspection tool
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// File to open
    file: String,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Read file contents
    let lines = match read_file(&cli.file) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", cli.file, e);
            std::process::exit(1);
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the TUI app
    let result = run_app(&mut terminal, &lines);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn read_file(path: impl AsRef<Path>) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    reader.lines().collect()
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    lines: &[String],
) -> io::Result<()> {
    // Calculate line number column width (like Vim's number gutter)
    let line_num_width = if lines.is_empty() {
        2 // Minimum width for "~" alignment
    } else {
        lines.len().to_string().len().max(1) + 1 // digits + trailing space
    };

    // Cursor position (0-indexed), like Vim
    let mut cursor_line: usize = 0;
    let mut cursor_col: usize = 0;
    // Vertical scroll offset: index of the first visible line
    let mut scroll_offset: usize = 0;
    // Horizontal scroll offset: index of the first visible character
    let mut scroll_col: usize = 0;

    loop {
        let size = terminal.size()?;
        let max_lines = size.height as usize;
        let max_width = size.width as usize;
        let content_width = max_width.saturating_sub(line_num_width + 1);

        // Keep cursor visible by adjusting vertical scroll offset
        if cursor_line < scroll_offset {
            scroll_offset = cursor_line;
        }
        if cursor_line >= scroll_offset + max_lines {
            scroll_offset = cursor_line - max_lines + 1;
        }

        // Keep cursor visible by adjusting horizontal scroll offset
        if content_width > 0 {
            if cursor_col < scroll_col {
                scroll_col = cursor_col;
            }
            if cursor_col >= scroll_col + content_width {
                scroll_col = cursor_col - content_width + 1;
            }
        } else {
            scroll_col = 0;
        }

        terminal.draw(|frame| {
            let mut rendered_lines: Vec<Line> = Vec::with_capacity(max_lines);

            for i in 0..max_lines {
                let file_line_idx = scroll_offset + i;

                if file_line_idx < lines.len() {
                    let is_cursor_line = file_line_idx == cursor_line;

                    // Line number (right-aligned, like Vim)
                    let line_num = format!("{:>width$}", file_line_idx + 1, width = line_num_width);

                    // File content, with horizontal scrolling
                    let content = &lines[file_line_idx];
                    let chars: Vec<char> = content.chars().collect();
                    let start = scroll_col.min(chars.len());
                    let end = (scroll_col + content_width).min(chars.len());

                    let mut spans = vec![
                        Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                    ];

                    if is_cursor_line && cursor_col >= start && cursor_col < end {
                        // Split the visible portion into before-cursor, cursor, and after-cursor
                        let rel_cursor = cursor_col - start;
                        let before: String = chars[start..start + rel_cursor].iter().collect();
                        let cursor_char = chars[start + rel_cursor].to_string();
                        let after: String = chars[start + rel_cursor + 1..end].iter().collect();

                        spans.push(Span::styled(before, Style::default().bg(Color::DarkGray)));
                        spans.push(Span::styled(
                            cursor_char,
                            Style::default().bg(Color::Yellow).fg(Color::Black),
                        ));
                        spans.push(Span::styled(after, Style::default().bg(Color::DarkGray)));
                    } else {
                        let display_content: String = chars[start..end].iter().collect();
                        let line_style = if is_cursor_line {
                            Style::default().bg(Color::DarkGray)
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(display_content, line_style));
                    }

                    rendered_lines.push(Line::from(spans));
                } else {
                    // Empty lines filled with ~ (like Vim)
                    let tilde = format!("{:>width$}", "~", width = line_num_width + 1);
                    rendered_lines.push(Line::from(Span::styled(
                        tilde,
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            frame.render_widget(Paragraph::new(rendered_lines), frame.area());
        })?;

        // Handle keyboard input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                // Down / j: move cursor down (like Vim)
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_line + 1 < lines.len() {
                        cursor_line += 1;
                        let line_len = lines[cursor_line].chars().count();
                        if line_len == 0 {
                            cursor_col = 0;
                        } else {
                            cursor_col = cursor_col.min(line_len - 1);
                        }
                    }
                }
                // Up / k: move cursor up (like Vim)
                KeyCode::Up | KeyCode::Char('k') => {
                    if cursor_line > 0 {
                        cursor_line -= 1;
                        let line_len = lines[cursor_line].chars().count();
                        if line_len == 0 {
                            cursor_col = 0;
                        } else {
                            cursor_col = cursor_col.min(line_len - 1);
                        }
                    }
                }
                // Left / h: move cursor left (like Vim)
                KeyCode::Left | KeyCode::Char('h') => {
                    if cursor_col > 0 {
                        cursor_col -= 1;
                    }
                }
                // Right / l: move cursor right (like Vim)
                KeyCode::Right | KeyCode::Char('l') => {
                    let line_len = lines[cursor_line].chars().count();
                    if line_len > 0 && cursor_col + 1 < line_len {
                        cursor_col += 1;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
