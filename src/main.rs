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

    // Cursor position (0-indexed line number in the file), like Vim
    let mut cursor: usize = 0;
    // Vertical scroll offset: index of the first visible line
    let mut scroll_offset: usize = 0;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let max_lines = area.height as usize;
            let max_width = area.width as usize;

            // Content area width (after line number column)
            let content_width = max_width.saturating_sub(line_num_width + 1);

            // Keep cursor visible by adjusting scroll offset
            if cursor < scroll_offset {
                scroll_offset = cursor;
            }
            if cursor >= scroll_offset + max_lines {
                scroll_offset = cursor - max_lines + 1;
            }

            let mut rendered_lines: Vec<Line> = Vec::with_capacity(max_lines);

            for i in 0..max_lines {
                let file_line_idx = scroll_offset + i;

                if file_line_idx < lines.len() {
                    let is_cursor_line = file_line_idx == cursor;

                    // Line number (right-aligned, like Vim)
                    let line_num = format!("{:>width$}", file_line_idx + 1, width = line_num_width);

                    // File content, truncated if too long (like Vim)
                    let content = &lines[file_line_idx];
                    let display_content = if content.len() > content_width {
                        content.chars().take(content_width).collect::<String>()
                    } else {
                        content.clone()
                    };

                    // Highlight the cursor line, similar to Vim's cursorline
                    let content_style = if is_cursor_line {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    rendered_lines.push(Line::from(vec![
                        Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(display_content, content_style),
                    ]));
                } else {
                    // Empty lines filled with ~ (like Vim)
                    let tilde = format!("{:>width$}", "~", width = line_num_width + 1);
                    rendered_lines.push(Line::from(Span::styled(
                        tilde,
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            frame.render_widget(Paragraph::new(rendered_lines), area);
        })?;

        // Handle keyboard input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                // j: move cursor down (like Vim)
                KeyCode::Char('j') => {
                    if cursor + 1 < lines.len() {
                        cursor += 1;
                    }
                }
                // k: move cursor up (like Vim)
                KeyCode::Char('k') => {
                    if cursor > 0 {
                        cursor -= 1;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
