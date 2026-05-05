use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
    /// Files to open
    files: Vec<String>,
}

/// Modal editing mode
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Command,
}

/// Saved cursor and scroll position for a buffer
#[derive(Clone, Copy)]
struct CursorState {
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    scroll_col: usize,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            scroll_col: 0,
        }
    }
}

/// Application state for the file viewer
struct AppState {
    buffers: Vec<Vec<String>>,
    buffer_names: Vec<String>,
    current_buffer: usize,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    scroll_col: usize,
    buffer_states: Vec<CursorState>,
    mode: Mode,
    command_buffer: String,
}

impl AppState {
    fn new(buffers: Vec<Vec<String>>, buffer_names: Vec<String>) -> Self {
        let count = buffers.len();
        Self {
            buffers,
            buffer_names,
            current_buffer: 0,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            scroll_col: 0,
            buffer_states: vec![CursorState::default(); count],
            mode: Mode::Normal,
            command_buffer: String::new(),
        }
    }

    /// Returns a reference to the current buffer's lines
    fn lines(&self) -> &Vec<String> {
        &self.buffers[self.current_buffer]
    }

    /// Returns a mutable reference to the current buffer's lines
    fn lines_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffers[self.current_buffer]
    }

    /// Save current cursor/scroll state and switch to another buffer
    fn switch_to_buffer(&mut self, new_index: usize) {
        // Save current position
        self.buffer_states[self.current_buffer] = CursorState {
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
            scroll_offset: self.scroll_offset,
            scroll_col: self.scroll_col,
        };
        self.current_buffer = new_index;
        // Restore target buffer's saved position
        let saved = self.buffer_states[self.current_buffer];
        self.cursor_line = saved.cursor_line;
        self.cursor_col = saved.cursor_col;
        self.scroll_offset = saved.scroll_offset;
        self.scroll_col = saved.scroll_col;
    }

    /// Width of the line number gutter (right-aligned, like Vim)
    fn line_num_width(&self) -> usize {
        let len = self.lines().len();
        if len == 0 {
            2 // Minimum width for "~" alignment
        } else {
            len.to_string().len().max(1) + 1 // digits + trailing space
        }
    }

    /// Adjust vertical and horizontal scroll offsets to keep cursor visible
    fn adjust_scroll(&mut self, file_area_height: usize, content_width: usize) {
        // Vertical
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
        if self.cursor_line >= self.scroll_offset + file_area_height {
            self.scroll_offset = self.cursor_line - file_area_height + 1;
        }

        // Horizontal
        if content_width > 0 {
            if self.cursor_col < self.scroll_col {
                self.scroll_col = self.cursor_col;
            }
            if self.cursor_col >= self.scroll_col + content_width {
                self.scroll_col = self.cursor_col - content_width + 1;
            }
        } else {
            self.scroll_col = 0;
        }
    }

    /// Clamp cursor within file bounds after cursor movement
    fn clamp_cursor(&mut self) {
        let num_lines = self.lines().len();
        if self.cursor_line >= num_lines {
            self.cursor_line = num_lines.saturating_sub(1);
        }
        let line_len = self.lines()[self.cursor_line].chars().count();
        if line_len == 0 {
            self.cursor_col = 0;
        } else {
            self.cursor_col = self.cursor_col.min(line_len - 1);
        }
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // If no files provided, read from stdin or show an error
    if cli.files.is_empty() {
        eprintln!("No files provided. Usage: fim <file1> [file2 ...]");
        std::process::exit(1);
    }

    // Read all file contents into buffers
    let mut buffers = Vec::with_capacity(cli.files.len());
    let mut buffer_names = Vec::with_capacity(cli.files.len());

    for filepath in &cli.files {
        match read_file(filepath) {
            Ok(lines) => {
                buffers.push(lines);
                buffer_names.push(filepath.clone());
            }
            Err(e) => {
                eprintln!("Error reading file '{}': {}", filepath, e);
                std::process::exit(1);
            }
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the TUI app
    let result = run_app(&mut terminal, buffers, buffer_names);

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
    buffers: Vec<Vec<String>>,
    buffer_names: Vec<String>,
) -> io::Result<()> {
    let mut state = AppState::new(buffers, buffer_names);

    loop {
        let size = terminal.size()?;
        let max_lines = size.height as usize;
        let line_num_width = state.line_num_width();
        // Reserve one line at the bottom for the command bar
        let file_area_height = max_lines.saturating_sub(1);
        let content_width = size.width as usize;
        let content_width = content_width.saturating_sub(line_num_width + 1);

        state.adjust_scroll(file_area_height, content_width);

        terminal.draw(|frame| {
            render(&state, frame);
        })?;

        if let Event::Key(key) = event::read()? {
            if state.mode == Mode::Normal {
                handle_normal_mode(&mut state, &key);
                if state.mode == Mode::Command && key.code == KeyCode::Char(':') {
                    // Already handled
                }
            } else {
                if handle_command_mode(&mut state, &key) {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn render(state: &AppState, frame: &mut ratatui::Frame) {
    let area = frame.area();
    let file_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let cmd_y = area.y.saturating_add(area.height.saturating_sub(1));
    let cmd_area = Rect::new(area.x, cmd_y, area.width, 1);

    let line_num_width = state.line_num_width();
    let file_area_height = area.height.saturating_sub(1) as usize;
    let content_width = area.width as usize;
    let content_width = content_width.saturating_sub(line_num_width + 1);

    // Render file content in the file area
    let mut rendered_lines: Vec<Line> = Vec::with_capacity(file_area_height);

    for i in 0..file_area_height {
        let file_line_idx = state.scroll_offset + i;

        if file_line_idx < state.lines().len() {
            let is_cursor_line = state.mode == Mode::Normal && file_line_idx == state.cursor_line;

            // Line number (right-aligned, like Vim)
            let line_num = format!("{:>width$}", file_line_idx + 1, width = line_num_width);

            // File content, with horizontal scrolling
            let content = &state.lines()[file_line_idx];
            let chars: Vec<char> = content.chars().collect();
            let start = state.scroll_col.min(chars.len());
            let end = (state.scroll_col + content_width).min(chars.len());

            let mut spans = vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
            ];

            if is_cursor_line && state.cursor_col >= start && state.cursor_col < end {
                // Split the visible portion into before-cursor, cursor, and after-cursor
                let rel_cursor = state.cursor_col - start;
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

    frame.render_widget(Paragraph::new(rendered_lines), file_area);

    // Render the command bar at the bottom
    let cmd_text = if state.mode == Mode::Command {
        format!(":{}", state.command_buffer)
    } else {
        String::new()
    };
    let cmd_style = Style::default().bg(Color::Black).fg(Color::White);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(cmd_text, cmd_style))),
        cmd_area,
    );
}

fn handle_normal_mode(state: &mut AppState, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char(':') => {
            state.mode = Mode::Command;
            state.command_buffer.clear();
        }
        // Down / j: move cursor down (like Vim)
        KeyCode::Down | KeyCode::Char('j') if state.cursor_line + 1 < state.lines().len() => {
            state.cursor_line += 1;
            state.clamp_cursor();
        }
        // Up / k: move cursor up (like Vim)
        KeyCode::Up | KeyCode::Char('k') if state.cursor_line > 0 => {
            state.cursor_line -= 1;
            state.clamp_cursor();
        }
        // Left / h: move cursor left (like Vim)
        KeyCode::Left | KeyCode::Char('h') if state.cursor_col > 0 => {
            state.cursor_col -= 1;
        }
        // Right / l: move cursor right (like Vim)
        KeyCode::Right | KeyCode::Char('l') => {
            let line_len = state.lines()[state.cursor_line].chars().count();
            if line_len > 0 && state.cursor_col + 1 < line_len {
                state.cursor_col += 1;
            }
        }
        _ => {}
    }
}

/// Returns true if the application should exit
fn handle_command_mode(state: &mut AppState, key: &crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(c) => {
            state.command_buffer.push(c);
        }
        KeyCode::Enter => {
            let cmd = std::mem::take(&mut state.command_buffer);
            state.mode = Mode::Normal;
            match cmd.as_str() {
                "q" => return true,
                "bn" => {
                    let new_idx = state.current_buffer + 1;
                    if new_idx < state.buffers.len() {
                        state.switch_to_buffer(new_idx);
                    }
                }
                "bp" => {
                    if state.current_buffer > 0 {
                        state.switch_to_buffer(state.current_buffer - 1);
                    }
                }
                _ => {}
            }
        }
        KeyCode::Esc => {
            state.command_buffer.clear();
            state.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            state.command_buffer.pop();
        }
        _ => {}
    }
    false
}
