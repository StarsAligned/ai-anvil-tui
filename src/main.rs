use std::env;
use std::path::PathBuf;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::runtime::Runtime;
use crate::ui::App;

mod ui;
mod input;
mod output;

fn main() {
    let rt = Runtime::new().unwrap();
    let mut args = env::args();
    let _ = args.next();
    let default_path = args.next().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string()
    });
    let default_output_path = format!(
        "{}{}merged_context.txt",
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("." ))
            .to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    rt.block_on(async {
        let mut app = App::new(default_path, default_output_path);
        app.reload_files_needed = true;
        enable_raw_mode().unwrap();
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        loop {
            if app.reload_files_needed && !app.processing {
                app.processing = true;
                terminal.draw(|f| app.draw(f)).unwrap();
                app.reload_files_immediate().await;
                app.processing = false;
            }
            if app.merge_needed && !app.processing {
                app.processing = true;
                terminal.draw(|f| app.draw(f)).unwrap();
                app.merge_immediate().await;
                app.processing = false;
            }
            terminal.draw(|f| app.draw(f)).unwrap();
            if app.exit_requested {
                break;
            }
            if event::poll(Duration::from_millis(50)).unwrap() {
                match event::read().unwrap() {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        app.update(key_event).await;
                    }
                    _ => {}
                }
            }
        }
        disable_raw_mode().unwrap();
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).unwrap();
        terminal.show_cursor().unwrap();
    });
}