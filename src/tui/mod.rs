use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};
use std::io;
use std::time::{Duration, Instant};

use crate::dashboard::Dashboard;
use crate::{DiffFile, HunkStatus, state::ReviewDb};

/// Filter mode for displaying hunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Unreviewed,
    Stale,
}

/// View mode for the TUI.
#[derive(Debug, Clone)]
pub enum ViewMode {
    Dashboard,
    HunkReview { branch: String, base_ref: String },
}

/// Confirmation action for bulk operations.
#[derive(Debug, Clone)]
enum ConfirmAction {
    ApproveAllFile { file_idx: usize },
    ApproveAll,
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
    scroll_offset: u16,
    highlighter: crate::highlight::Highlighter,
    confirm_action: Option<ConfirmAction>,
    pub view_mode: ViewMode,
    pub dashboard: Option<Dashboard>,
    status_message: Option<(String, Instant)>,
    last_refresh: Instant,
}

impl App {
    /// Create a new App for hunk review mode.
    ///
    /// Syncs files with the database and loads review status.
    pub fn new_hunk_review(
        files: Vec<DiffFile>,
        mut db: ReviewDb,
        base_ref: String,
    ) -> Result<Self> {
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

        Ok(Self {
            files,
            db,
            base_ref: base_ref.clone(),
            selected_file: 0,
            selected_hunk: 0,
            filter: FilterMode::All,
            should_quit: false,
            show_help: false,
            scroll_offset: 0,
            highlighter: crate::highlight::Highlighter::new(),
            confirm_action: None,
            view_mode: ViewMode::HunkReview {
                branch: String::new(),
                base_ref,
            },
            dashboard: None,
            status_message: None,
            last_refresh: Instant::now(),
        })
    }

    /// Create a new App for dashboard mode.
    ///
    /// Loads all branches and their review progress.
    pub fn new_dashboard(db: ReviewDb, base_branch: String) -> Result<Self> {
        let dashboard = Dashboard::load(&db, &base_branch)
            .map_err(|e| anyhow::anyhow!("Failed to load dashboard: {}", e))?;

        Ok(Self {
            files: vec![],
            db,
            base_ref: base_branch,
            selected_file: 0,
            selected_hunk: 0,
            filter: FilterMode::All,
            should_quit: false,
            show_help: false,
            scroll_offset: 0,
            highlighter: crate::highlight::Highlighter::new(),
            confirm_action: None,
            view_mode: ViewMode::Dashboard,
            dashboard: Some(dashboard),
            status_message: None,
            last_refresh: Instant::now(),
        })
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

    /// Handle keyboard input, dispatching to the appropriate mode handler.
    fn handle_input(&mut self, key: event::KeyEvent) -> Result<()> {
        // Handle confirmation dialog first
        if let Some(action) = self.confirm_action.take() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => match action {
                    ConfirmAction::ApproveAllFile { file_idx } => {
                        self.selected_file = file_idx;
                        self.approve_current_file()?;
                    }
                    ConfirmAction::ApproveAll => {
                        self.approve_all()?;
                    }
                },
                _ => {} // Any other key cancels
            }
            return Ok(());
        }

        if self.show_help {
            // Any key closes help
            self.show_help = false;
            return Ok(());
        }

        match self.view_mode {
            ViewMode::Dashboard => self.handle_dashboard_input(key),
            ViewMode::HunkReview { .. } => self.handle_hunk_review_input(key),
        }
    }

    /// Handle keyboard input in dashboard mode.
    fn handle_dashboard_input(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(ref mut dashboard) = self.dashboard {
                    dashboard.select_next();
                    let _ = dashboard.load_detail_for_selected(&self.db);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(ref mut dashboard) = self.dashboard {
                    dashboard.select_prev();
                    let _ = dashboard.load_detail_for_selected(&self.db);
                }
            }
            KeyCode::Enter => {
                // Placeholder for drill-in (Step 3.1 will implement)
                self.status_message =
                    Some(("Drill-in not yet implemented".to_string(), Instant::now()));
            }
            KeyCode::Char('M') => {
                // Placeholder for merge (Step 4.1 will implement)
                self.status_message =
                    Some(("Merge not yet implemented".to_string(), Instant::now()));
            }
            KeyCode::Char('r') => {
                self.try_refresh_dashboard();
                self.last_refresh = Instant::now();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keyboard input in hunk review mode.
    fn handle_hunk_review_input(&mut self, key: event::KeyEvent) -> Result<()> {
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
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
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
            KeyCode::Char('F') => {
                // Shift+F: approve current file (with confirmation)
                if self.selected_file < self.files.len() {
                    self.confirm_action = Some(ConfirmAction::ApproveAllFile {
                        file_idx: self.selected_file,
                    });
                }
            }
            KeyCode::Char('A') => {
                // Shift+A: approve all (with confirmation)
                if !self.files.is_empty() {
                    self.confirm_action = Some(ConfirmAction::ApproveAll);
                }
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(20);
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
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
                self.scroll_offset = 0;
            }
        } else if !visible.is_empty() {
            self.selected_hunk = visible[0];
            self.scroll_offset = 0;
        }
    }

    /// Navigate to the previous hunk.
    fn navigate_hunk_up(&mut self) {
        self.scroll_offset = 0;
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
        self.scroll_offset = 0;
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

    /// Approve all hunks in the currently selected file.
    fn approve_current_file(&mut self) -> Result<()> {
        if self.selected_file >= self.files.len() {
            return Ok(());
        }
        let file = &self.files[self.selected_file];
        let file_path = file.path.to_string_lossy().to_string();
        // Collect hashes to approve
        let to_approve: Vec<(String, usize)> = file
            .hunks
            .iter()
            .enumerate()
            .filter(|(_, h)| h.status != HunkStatus::Reviewed)
            .map(|(i, h)| (h.content_hash.clone(), i))
            .collect();
        // Update DB
        for (hash, _) in &to_approve {
            self.db
                .set_status(&self.base_ref, &file_path, hash, HunkStatus::Reviewed)
                .context("Failed to approve hunk")?;
        }
        // Update in-memory state
        let file = &mut self.files[self.selected_file];
        for (_, idx) in &to_approve {
            file.hunks[*idx].status = HunkStatus::Reviewed;
        }
        Ok(())
    }

    /// Approve all hunks in all files.
    fn approve_all(&mut self) -> Result<()> {
        // Collect all hunks to approve
        let mut to_approve: Vec<(usize, usize, String, String)> = Vec::new();
        for (file_idx, file) in self.files.iter().enumerate() {
            let file_path = file.path.to_string_lossy().to_string();
            for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
                if hunk.status != HunkStatus::Reviewed {
                    to_approve.push((
                        file_idx,
                        hunk_idx,
                        file_path.clone(),
                        hunk.content_hash.clone(),
                    ));
                }
            }
        }
        // Update DB
        for (_, _, file_path, hash) in &to_approve {
            self.db
                .set_status(&self.base_ref, file_path, hash, HunkStatus::Reviewed)
                .context("Failed to approve hunk")?;
        }
        // Update in-memory state
        for (file_idx, hunk_idx, _, _) in &to_approve {
            self.files[*file_idx].hunks[*hunk_idx].status = HunkStatus::Reviewed;
        }
        Ok(())
    }

    /// Attempt to refresh the dashboard from git state.
    fn try_refresh_dashboard(&mut self) {
        if let Some(ref mut dashboard) = self.dashboard {
            match dashboard.refresh(&self.db) {
                Ok(true) => {
                    let _ = dashboard.load_detail_for_selected(&self.db);
                }
                Ok(false) => {}
                Err(e) => {
                    self.status_message = Some((format!("Refresh failed: {}", e), Instant::now()));
                }
            }
        }
    }

    /// Render the UI, dispatching to the appropriate mode renderer.
    fn render(&mut self, frame: &mut Frame) {
        // Expire old status messages
        let expired = self
            .status_message
            .as_ref()
            .map(|(_, time)| time.elapsed() >= Duration::from_secs(3))
            .unwrap_or(false);
        if expired {
            self.status_message = None;
        }

        if self.show_help {
            self.render_help(frame);
            return;
        }

        match self.view_mode {
            ViewMode::Dashboard => self.render_dashboard(frame),
            ViewMode::HunkReview { .. } => self.render_hunk_review(frame),
        }

        // Draw confirmation modal on top if active
        if self.confirm_action.is_some() {
            self.render_confirm(frame);
        }
    }

    /// Render the dashboard view with branch table.
    fn render_dashboard(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(frame.area());

        let dashboard = match &self.dashboard {
            Some(d) => d,
            None => return,
        };

        let rows: Vec<Row> = dashboard
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = idx == dashboard.selected;
                let prefix = if is_selected { ">" } else { " " };
                let branch_name = &item.branch.name;

                let diff_str = match &item.detail {
                    Some(d) => format!("+{}/-{}", d.diff_stats.insertions, d.diff_stats.deletions),
                    None => "-".to_string(),
                };

                let files_str = match &item.detail {
                    Some(d) => d.diff_stats.file_count.to_string(),
                    None => "-".to_string(),
                };

                let review_str = match &item.progress {
                    Some(p) if p.total > 0 => {
                        format!("{:.0}%", (p.reviewed as f64 / p.total as f64) * 100.0)
                    }
                    _ => "-".to_string(),
                };

                let commit_str = &item.branch.last_commit_age;

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(format!("{} {}", prefix, branch_name)),
                    Cell::from(diff_str),
                    Cell::from(files_str),
                    Cell::from(review_str),
                    Cell::from(commit_str.clone()),
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ];

        let header = Row::new(vec!["Branch", "+/-", "Files", "Review", "Commit"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let table = Table::new(rows, widths)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Branch Dashboard"),
            )
            .header(header);

        frame.render_widget(table, chunks[0]);

        // Status bar
        let status_text = match &self.status_message {
            Some((msg, _)) => msg.clone(),
            None => {
                let count = dashboard.items.len();
                format!(
                    "{} branches | j/k: navigate  Enter: review  M: merge  r: refresh  q: quit",
                    count
                )
            }
        };

        let status_bar = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });

        frame.render_widget(status_bar, chunks[1]);
    }

    /// Render the hunk review view (existing behavior).
    fn render_hunk_review(&self, frame: &mut Frame) {
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
        let file_ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let mut fh = self.highlighter.for_file(file_ext);
        for line in hunk.content.lines() {
            let spans = fh.highlight_diff_line(line);
            lines.push(Line::from(spans));
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
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

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
            "{}/{} hunks reviewed ({} stale), {} files remaining | Filter: {} | Keys: j/k=nav Space=toggle F=approve-file A=approve-all Tab=file u/s/a=filter ?=help q=quit",
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
        let help_text: Vec<&str> = match self.view_mode {
            ViewMode::Dashboard => vec![
                "Git Review - Dashboard Shortcuts",
                "",
                "Navigation:",
                "  j / Down      - Next branch",
                "  k / Up        - Previous branch",
                "",
                "Actions:",
                "  Enter         - Review selected branch",
                "  M (Shift+M)   - Merge selected branch",
                "  r             - Refresh branch list",
                "",
                "Other:",
                "  ?             - Show this help",
                "  q / Esc       - Quit",
                "",
                "Press any key to close this help",
            ],
            ViewMode::HunkReview { .. } => vec![
                "Git Review - Keyboard Shortcuts",
                "",
                "Navigation:",
                "  j / Down      - Next hunk",
                "  k / Up        - Previous hunk",
                "  Tab           - Next file",
                "  Shift+Tab     - Previous file",
                "  Ctrl+d/PgDn  - Scroll down",
                "  Ctrl+u/PgUp  - Scroll up",
                "",
                "Actions:",
                "  Space         - Toggle reviewed status",
                "",
                "Bulk Actions:",
                "  F (Shift+F)   - Approve all hunks in current file",
                "  A (Shift+A)   - Approve all hunks in all files",
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
            ],
        };

        let text = Text::from(help_text.iter().map(|&s| Line::from(s)).collect::<Vec<_>>());

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: false });

        let area = centered_rect(60, 80, frame.area());
        frame.render_widget(paragraph, area);
    }

    /// Render the confirmation modal.
    fn render_confirm(&self, frame: &mut Frame) {
        let message = match &self.confirm_action {
            Some(ConfirmAction::ApproveAllFile { file_idx }) => {
                let file_path = self.files[*file_idx].path.to_string_lossy();
                let count = self.files[*file_idx]
                    .hunks
                    .iter()
                    .filter(|h| h.status != HunkStatus::Reviewed)
                    .count();
                format!(
                    "Approve {} unreviewed hunks in {}?\n\n(y)es / (n)o",
                    count, file_path
                )
            }
            Some(ConfirmAction::ApproveAll) => {
                let count: usize = self
                    .files
                    .iter()
                    .flat_map(|f| &f.hunks)
                    .filter(|h| h.status != HunkStatus::Reviewed)
                    .count();
                format!(
                    "Approve {} unreviewed hunks in all files?\n\n(y)es / (n)o",
                    count
                )
            }
            None => return,
        };

        let paragraph = Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL).title("Confirm"))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Yellow));

        let area = centered_rect(50, 30, frame.area());
        // Clear the area first
        frame.render_widget(Clear, area);
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
///
/// Accepts a pre-configured App (created via `App::new_hunk_review` or `App::new_dashboard`).
pub fn run_tui(mut app: App) -> Result<()> {
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut terminal = setup_terminal()?;

    // Main event loop
    let result = (|| -> Result<()> {
        loop {
            terminal
                .draw(|f| app.render(f))
                .context("Failed to draw frame")?;

            if app.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(200)).context("Failed to poll events")?
                && let Event::Key(key) = event::read().context("Failed to read event")?
            {
                // Ignore key release events
                if key.kind == event::KeyEventKind::Press {
                    app.handle_input(key)?;
                }
            }

            // Auto-refresh in dashboard mode (every 5 seconds)
            if matches!(app.view_mode, ViewMode::Dashboard)
                && app.last_refresh.elapsed() >= Duration::from_secs(5)
            {
                app.try_refresh_dashboard();
                app.last_refresh = Instant::now();
            }
        }
        Ok(())
    })();

    // Restore terminal in all cases
    restore_terminal(&mut terminal)?;

    result
}
