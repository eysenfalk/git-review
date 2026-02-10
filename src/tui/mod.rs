use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io;

use crate::{DiffFile, HunkStatus, state::ReviewDb};

/// Filter mode for displaying hunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Unreviewed,
    Stale,
}

/// Application state for the TUI.
pub struct App {
    files: Vec<DiffFile>,
    db: ReviewDb,
    base_ref: String,
    selected_file: usize,
    selected_hunk: usize,
    filter: FilterMode,
    should_quit: bool,
    show_help: bool,
}

impl App {
    /// Create a new App instance.
    fn new(files: Vec<DiffFile>, db: ReviewDb, base_ref: String) -> Self {
        Self {
            files,
            db,
            base_ref,
            selected_file: 0,
            selected_hunk: 0,
            filter: FilterMode::All,
            should_quit: false,
            show_help: false,
        }
    }

    /// Get currently visible files based on filter mode.
    fn visible_files(&self) -> Vec<usize> {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, file)| {
                file.hunks.iter().any(|hunk| match self.filter {
                    FilterMode::All => true,
                    FilterMode::Unreviewed => hunk.status == HunkStatus::Unreviewed,
                    FilterMode::Stale => hunk.status == HunkStatus::Stale,
                })
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Get currently visible hunks for the selected file.
    fn visible_hunks(&self) -> Vec<usize> {
        if self.selected_file >= self.files.len() {
            return Vec::new();
        }
        self.files[self.selected_file]
            .hunks
            .iter()
            .enumerate()
            .filter(|(_, hunk)| match self.filter {
                FilterMode::All => true,
                FilterMode::Unreviewed => hunk.status == HunkStatus::Unreviewed,
                FilterMode::Stale => hunk.status == HunkStatus::Stale,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Handle keyboard input.
    fn handle_input(&mut self, key: event::KeyEvent) -> Result<()> {
        if self.show_help {
            // Any key closes help
            self.show_help = false;
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.navigate_hunk_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.navigate_hunk_up();
            }
            KeyCode::Tab => {
                self.navigate_file_next();
            }
            KeyCode::BackTab => {
                self.navigate_file_prev();
            }
            KeyCode::Char(' ') => {
                self.toggle_reviewed()?;
            }
            KeyCode::Char('u') => {
                self.filter = FilterMode::Unreviewed;
                self.reset_selection();
            }
            KeyCode::Char('s') => {
                self.filter = FilterMode::Stale;
                self.reset_selection();
            }
            KeyCode::Char('a') => {
                self.filter = FilterMode::All;
                self.reset_selection();
            }
            _ => {}
        }
        Ok(())
    }

    /// Navigate to the next hunk.
    fn navigate_hunk_down(&mut self) {
        let visible = self.visible_hunks();
        if visible.is_empty() {
            return;
        }
        if let Some(current_pos) = visible.iter().position(|&i| i == self.selected_hunk) {
            if current_pos + 1 < visible.len() {
                self.selected_hunk = visible[current_pos + 1];
            }
        } else if !visible.is_empty() {
            self.selected_hunk = visible[0];
        }
    }

    /// Navigate to the previous hunk.
    fn navigate_hunk_up(&mut self) {
        let visible = self.visible_hunks();
        if visible.is_empty() {
            return;
        }
        if let Some(current_pos) = visible.iter().position(|&i| i == self.selected_hunk) {
            if current_pos > 0 {
                self.selected_hunk = visible[current_pos - 1];
            }
        } else if !visible.is_empty() {
            self.selected_hunk = visible[0];
        }
    }

    /// Navigate to the next file.
    fn navigate_file_next(&mut self) {
        let visible = self.visible_files();
        if visible.is_empty() {
            return;
        }
        if let Some(current_pos) = visible.iter().position(|&i| i == self.selected_file)
            && current_pos + 1 < visible.len()
        {
            self.selected_file = visible[current_pos + 1];
            self.reset_hunk_selection();
        }
    }

    /// Navigate to the previous file.
    fn navigate_file_prev(&mut self) {
        let visible = self.visible_files();
        if visible.is_empty() {
            return;
        }
        if let Some(current_pos) = visible.iter().position(|&i| i == self.selected_file)
            && current_pos > 0
        {
            self.selected_file = visible[current_pos - 1];
            self.reset_hunk_selection();
        }
    }

    /// Reset hunk selection to first visible hunk.
    fn reset_hunk_selection(&mut self) {
        let visible = self.visible_hunks();
        self.selected_hunk = visible.first().copied().unwrap_or(0);
    }

    /// Reset selection after filter change.
    fn reset_selection(&mut self) {
        let visible_files = self.visible_files();
        self.selected_file = visible_files.first().copied().unwrap_or(0);
        self.reset_hunk_selection();
    }

    /// Toggle the reviewed status of the current hunk.
    fn toggle_reviewed(&mut self) -> Result<()> {
        if self.selected_file >= self.files.len() {
            return Ok(());
        }
        let file = &mut self.files[self.selected_file];
        if self.selected_hunk >= file.hunks.len() {
            return Ok(());
        }

        let hunk = &mut file.hunks[self.selected_hunk];
        let file_path = file.path.to_string_lossy();

        let new_status = match hunk.status {
            HunkStatus::Unreviewed | HunkStatus::Stale => HunkStatus::Reviewed,
            HunkStatus::Reviewed => HunkStatus::Unreviewed,
        };

        self.db
            .set_status(&self.base_ref, &file_path, &hunk.content_hash, new_status)
            .context("Failed to update hunk status")?;

        hunk.status = new_status;
        Ok(())
    }

    /// Render the UI.
    fn render(&mut self, frame: &mut Frame) {
        if self.show_help {
            self.render_help(frame);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
            .split(frame.area());

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(chunks[0]);

        self.render_file_list(frame, main_chunks[0]);
        self.render_hunk_detail(frame, main_chunks[1]);
        self.render_status_bar(frame, chunks[1]);
    }

    /// Render the file list panel.
    fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        let visible = self.visible_files();
        let items: Vec<ListItem> = visible
            .iter()
            .map(|&file_idx| {
                let file = &self.files[file_idx];
                let file_path = file.path.to_string_lossy();

                let (reviewed, total) = file.hunks.iter().fold((0, 0), |(r, t), hunk| {
                    let include = match self.filter {
                        FilterMode::All => true,
                        FilterMode::Unreviewed => hunk.status == HunkStatus::Unreviewed,
                        FilterMode::Stale => hunk.status == HunkStatus::Stale,
                    };
                    if include {
                        let r = if hunk.status == HunkStatus::Reviewed {
                            r + 1
                        } else {
                            r
                        };
                        (r, t + 1)
                    } else {
                        (r, t)
                    }
                });

                let color = if reviewed == total && total > 0 {
                    Color::Green
                } else if reviewed > 0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let style = if file_idx == self.selected_file {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                ListItem::new(format!("{} ({}/{})", file_path, reviewed, total)).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Files (Tab/Shift+Tab)"),
        );

        frame.render_widget(list, area);
    }

    /// Render the hunk detail panel.
    fn render_hunk_detail(&self, frame: &mut Frame, area: Rect) {
        if self.selected_file >= self.files.len() {
            let paragraph = Paragraph::new("No file selected")
                .block(Block::default().borders(Borders::ALL).title("Hunk Detail"));
            frame.render_widget(paragraph, area);
            return;
        }

        let file = &self.files[self.selected_file];
        if self.selected_hunk >= file.hunks.len() {
            let paragraph = Paragraph::new("No hunk selected")
                .block(Block::default().borders(Borders::ALL).title("Hunk Detail"));
            frame.render_widget(paragraph, area);
            return;
        }

        let hunk = &file.hunks[self.selected_hunk];

        let mut lines = Vec::new();

        // Add hunk header
        let header = format!(
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        );
        lines.push(Line::from(Span::styled(
            header,
            Style::default().fg(Color::Cyan),
        )));

        // Add hunk content with syntax highlighting
        for line in hunk.content.lines() {
            let styled_line = if line.starts_with('+') {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else if line.starts_with('-') {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else {
                Line::from(line)
            };
            lines.push(styled_line);
        }

        let status_str = match hunk.status {
            HunkStatus::Reviewed => " [REVIEWED]",
            HunkStatus::Unreviewed => " [UNREVIEWED]",
            HunkStatus::Stale => " [STALE]",
        };

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Hunk Detail (Space to toggle){}", status_str)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Render the status bar.
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let progress = self
            .db
            .progress(&self.base_ref)
            .unwrap_or(crate::ReviewProgress {
                total_hunks: 0,
                reviewed: 0,
                unreviewed: 0,
                stale: 0,
                files_remaining: 0,
                total_files: 0,
            });

        let filter_str = match self.filter {
            FilterMode::All => "All",
            FilterMode::Unreviewed => "Unreviewed",
            FilterMode::Stale => "Stale",
        };

        let status_text = format!(
            "{}/{} hunks reviewed ({} stale), {} files remaining | Filter: {} | Keys: j/k=nav Space=toggle Tab=file u/s/a=filter ?=help q=quit",
            progress.reviewed,
            progress.total_hunks,
            progress.stale,
            progress.files_remaining,
            filter_str
        );

        let paragraph = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Render the help overlay.
    fn render_help(&self, frame: &mut Frame) {
        let help_text = vec![
            "Git Review - Keyboard Shortcuts",
            "",
            "Navigation:",
            "  j / Down      - Next hunk",
            "  k / Up        - Previous hunk",
            "  Tab           - Next file",
            "  Shift+Tab     - Previous file",
            "",
            "Actions:",
            "  Space         - Toggle reviewed status",
            "",
            "Filters:",
            "  u             - Show unreviewed hunks only",
            "  s             - Show stale hunks only",
            "  a             - Show all hunks",
            "",
            "Other:",
            "  ?             - Show this help",
            "  q / Esc       - Quit",
            "",
            "Press any key to close this help",
        ];

        let text = Text::from(help_text.iter().map(|&s| Line::from(s)).collect::<Vec<_>>());

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: false });

        let area = centered_rect(60, 80, frame.area());
        frame.render_widget(paragraph, area);
    }
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Setup the terminal for TUI rendering.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("Failed to create terminal")
}

/// Restore the terminal to its original state.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

/// Launch the interactive TUI review interface.
pub fn run_tui(files: Vec<DiffFile>, mut db: ReviewDb, base_ref: String) -> Result<()> {
    // Sync files with database
    db.sync_with_diff(&base_ref, &files)
        .context("Failed to sync with database")?;

    // Update file hunks with database status
    let mut files = files;
    for file in &mut files {
        let file_path = file.path.to_string_lossy();
        for hunk in &mut file.hunks {
            if let Ok(status) = db.get_status(&base_ref, &file_path, &hunk.content_hash) {
                hunk.status = status;
            }
        }
    }

    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut terminal = setup_terminal()?;
    let mut app = App::new(files, db, base_ref);

    // Main event loop
    let result = (|| -> Result<()> {
        loop {
            terminal
                .draw(|f| app.render(f))
                .context("Failed to draw frame")?;

            if app.should_quit {
                break;
            }

            if event::poll(std::time::Duration::from_millis(100))
                .context("Failed to poll events")?
                && let Event::Key(key) = event::read().context("Failed to read event")?
            {
                // Ignore key release events
                if key.kind == event::KeyEventKind::Press {
                    app.handle_input(key)?;
                }
            }
        }
        Ok(())
    })();

    // Restore terminal in all cases
    restore_terminal(&mut terminal)?;

    result
}
