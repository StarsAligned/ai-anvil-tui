use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use walkdir::WalkDir;

struct App {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    list_state: ListState,
}

impl App {
    fn new() -> Self {
        let current_path = std::env::current_dir().unwrap_or_default();
        let mut app = Self {
            current_path,
            entries: Vec::new(),
            list_state: ListState::default(),
        };
        app.refresh_entries();
        app.list_state.select(Some(0));
        app
    }

    fn refresh_entries(&mut self) {
        self.entries.clear();
        for entry in WalkDir::new(&self.current_path)
            .min_depth(1)
            .max_depth(1)
            .sort_by_file_name()
        {
            if let Ok(entry) = entry {
                self.entries.push(entry.path().to_path_buf());
            }
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.entries.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn enter_directory(&mut self) -> bool {
        if let Some(selected) = self.list_state.selected() {
            if let Some(path) = self.entries.get(selected) {
                if path.is_dir() {
                    self.current_path = path.clone();
                    self.refresh_entries();
                    self.list_state.select(Some(0));
                    return true;
                }
            }
        }
        false
    }

    fn go_up(&mut self) -> bool {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.refresh_entries();
            self.list_state.select(Some(0));
            return true;
        }
        false
    }

    fn get_selected_entry_type(&self) -> Option<EntryType> {
        self.list_state.selected().and_then(|selected| {
            self.entries.get(selected).map(|path| {
                if path.is_dir() {
                    EntryType::Directory
                } else {
                    EntryType::File
                }
            })
        })
    }
}

#[derive(PartialEq)]
enum EntryType {
    File,
    Directory,
}

fn format_entry(path: &Path, current_path: &Path) -> String {
    let relative = path.strip_prefix(current_path).unwrap_or(path);
    let prefix = if path.is_dir() { "📁 " } else { "📄 " };
    format!("{}{}", prefix, relative.display())
}

fn ui(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Path display
            Constraint::Min(0),    // File list
            Constraint::Length(3), // Help text
        ])
        .split(area);

    // Current path display
    let current_path = Paragraph::new(format!("Current path: {}", app.current_path.display()))
        .block(Block::default().borders(Borders::ALL).title("Directory"));
    frame.render_widget(current_path, chunks[0]);

    // File list
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|path| {
            ListItem::new(format_entry(path, &app.current_path)).style(Style::default().fg(if path.is_dir() {
                Color::Cyan
            } else {
                Color::White
            }))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Help text
    let help_text = match app.get_selected_entry_type() {
        Some(EntryType::Directory) => "↑/↓: Navigate  →/Enter: Open Directory  ←/Esc: Go Up  q: Quit",
        Some(EntryType::File) => "↑/↓: Navigate  ←/Esc: Go Up  q: Quit",
        None => "↑/↓: Navigate  q: Quit",
    };
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[2]);
}

fn run_app(terminal: &mut Terminal<impl Backend>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Enter | KeyCode::Right => {
                            if app.get_selected_entry_type() == Some(EntryType::Directory) {
                                app.enter_directory();
                            }
                        }
                        KeyCode::Left | KeyCode::Esc => {
                            app.go_up();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}