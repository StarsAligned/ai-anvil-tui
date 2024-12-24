use std::collections::HashSet;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use crate::input::{create_text_source, FilterConfig, SourceFile};
use crate::output::{write_merged, clipboard::copy_clipboard};
use crate::ui::output::{OutputDestination, OutputPanel};

pub mod source_path;
pub mod filters;
pub mod source_files;
pub mod output_file;
pub mod output;

#[derive(Copy, Clone, PartialEq)]
pub enum FocusedPanel {
    SourcePath,
    Filters,
    SourceFiles,
    Output,
    OutputFile,
}

impl FocusedPanel {
    pub fn next_panel(&self, app: &App) -> Self {
        match self {
            FocusedPanel::SourcePath => FocusedPanel::Filters,
            FocusedPanel::Filters => FocusedPanel::SourceFiles,
            FocusedPanel::SourceFiles => FocusedPanel::Output,
            FocusedPanel::Output => {
                if app.output_panel.destination == OutputDestination::Clipboard {
                    FocusedPanel::SourcePath
                } else {
                    FocusedPanel::OutputFile
                }
            }
            FocusedPanel::OutputFile => FocusedPanel::SourcePath,
        }
    }
    pub fn prev_panel(&self, app: &App) -> Self {
        match self {
            FocusedPanel::SourcePath => FocusedPanel::OutputFile,
            FocusedPanel::Filters => FocusedPanel::SourcePath,
            FocusedPanel::SourceFiles => FocusedPanel::Filters,
            FocusedPanel::Output => FocusedPanel::SourceFiles,
            FocusedPanel::OutputFile => {
                if app.output_panel.destination == OutputDestination::Clipboard {
                    FocusedPanel::Output
                } else {
                    FocusedPanel::Output
                }
            }
        }
    }
}

pub struct App {
    pub source_path_panel: source_path::SourcePathPanel,
    pub filters_panel: filters::FiltersPanel,
    pub source_files_panel: source_files::SourceFilesPanel,
    pub output_panel: OutputPanel,
    pub output_file_panel: output_file::OutputFilePanel,
    pub focused_panel: FocusedPanel,
    pub loaded_files: Vec<SourceFile>,
    pub selected_extensions: HashSet<String>,
    pub selected_files: HashSet<String>,
    pub processing: bool,
    pub filter_config: FilterConfig,
    pub text_source: Option<Box<dyn crate::input::TextSource>>,
    pub exit_requested: bool,
    pub reload_files_needed: bool,
    pub merge_needed: bool,
    pub prev_source_path: String,
}

impl App {
    pub fn new(default_path: String, default_output_path: String) -> Self {
        Self {
            source_path_panel: source_path::SourcePathPanel::new(default_path.clone()),
            filters_panel: filters::FiltersPanel::new(),
            source_files_panel: source_files::SourceFilesPanel::new(),
            output_panel: OutputPanel::new(),
            output_file_panel: output_file::OutputFilePanel::new(default_output_path),
            focused_panel: FocusedPanel::SourcePath,
            loaded_files: Vec::new(),
            selected_extensions: HashSet::new(),
            selected_files: HashSet::new(),
            processing: false,
            filter_config: FilterConfig::new(),
            text_source: None,
            exit_requested: false,
            reload_files_needed: false,
            merge_needed: false,
            prev_source_path: default_path,
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let show_output_file = self.output_panel.destination != OutputDestination::Clipboard;
        let mut row_constraints = vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ];
        if !show_output_file {
            row_constraints[3] = Constraint::Length(0);
        }
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(f.area());

        self.source_path_panel.draw(
            f,
            main_chunks[0],
            self.focused_panel == FocusedPanel::SourcePath
        );

        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(10)])
            .split(main_chunks[1]);

        self.filters_panel.draw(
            f,
            mid[0],
            self.focused_panel == FocusedPanel::Filters,
            &self.selected_extensions
        );
        self.source_files_panel.draw(
            f,
            mid[1],
            self.focused_panel == FocusedPanel::SourceFiles,
            &self.selected_files
        );
        self.output_panel.draw(
            f,
            main_chunks[2],
            self.focused_panel == FocusedPanel::Output
        );

        if show_output_file {
            self.output_file_panel.draw(
                f,
                main_chunks[3],
                self.focused_panel == FocusedPanel::OutputFile
            );
        }

        let paragraph = Paragraph::new(self.get_bottom_text())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(paragraph, main_chunks[4]);

        if self.processing {
            self.draw_overlay(f);
        }
    }

    fn get_bottom_text(&self) -> String {
        match self.focused_panel {
            FocusedPanel::SourcePath => "enter - Filters  •  F1 - reload  •  F2 - merge  •  F3 - clear  •  F10/esc - close".to_string(),
            FocusedPanel::Filters => "↑/↓ - navigate  •  space - (de)select  •  enter - Files  •  esc - Source  •  F1 - reload  •  F2 - generate  •  F10 - close".to_string(),
            FocusedPanel::SourceFiles => "↑/↓ - navigate  •  space - (de)select  •  enter - Output  •  esc - Filters  •  F1 - reload  •  F2 - generate  •  F10 - close".to_string(),
            FocusedPanel::Output => {
                match self.output_panel.destination {
                    OutputDestination::File | OutputDestination::FileAndClipboard => {
                        "←/→ - toggle  •  enter - Output File  •  esc - Files  •  F1 - reload  •  F2 - generate  •  F10 - close".to_string()
                    }
                    OutputDestination::Clipboard => {
                        "←/→ - toggle  •  enter/F2 - merge  •  esc - Files  •  F1 - reload  •  F10 - close".to_string()
                    }
                }
            }
            FocusedPanel::OutputFile => "enter/F2 - merge  •  esc - Output  •  F1 - reload  •  F3 - clear  •  F10 - close".to_string()
        }
    }

    fn draw_overlay(&self, f: &mut Frame) {
        let overlay_rect = f.area();
        let overlay_area = self.centered_rect(30, 5, overlay_rect);
        let overlay_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));
        f.render_widget(Clear, overlay_area);
        f.render_widget(overlay_block.clone(), overlay_area);
        let inner = overlay_block.inner(overlay_area);
        let overlay_text = Paragraph::new("Processing...");
        f.render_widget(overlay_text, inner);
    }

    fn centered_rect(&self, width: u16, height: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
        let left = r.x + (r.width.saturating_sub(width)) / 2;
        let top = r.y + (r.height.saturating_sub(height)) / 2;
        ratatui::layout::Rect {
            x: left,
            y: top,
            width: width.min(r.width),
            height: height.min(r.height),
        }
    }

    pub async fn update(&mut self, key_event: KeyEvent) {
        let old_focused_panel = self.focused_panel;
        match key_event.code {
            KeyCode::F(n) if n == 10 => {
                self.exit_requested = true;
            }
            KeyCode::Esc => {
                if self.focused_panel == FocusedPanel::SourcePath {
                    self.exit_requested = true;
                } else {
                    self.focused_panel = self.focused_panel.prev_panel(self);
                    self.set_cursor_to_end();
                }
            }
            KeyCode::F(n) if n == 1 => {
                if !self.processing {
                    self.reload_files_needed = true;
                }
            }
            KeyCode::F(n) if n == 2 => {
                if !self.processing {
                    self.merge_needed = true;
                }
            }
            KeyCode::F(n) if n == 3 => {
                match self.focused_panel {
                    FocusedPanel::SourcePath => {
                        self.source_path_panel.value.clear();
                        self.source_path_panel.cursor_pos = 0;
                    }
                    FocusedPanel::OutputFile => {
                        self.output_file_panel.value.clear();
                        self.output_file_panel.cursor_pos = 0;
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                self.handle_enter().await;
            }
            KeyCode::Char(' ') => {
                match self.focused_panel {
                    FocusedPanel::Filters => {
                        self.filters_panel.toggle_selected(
                            &mut self.selected_extensions,
                            &mut self.selected_files,
                            &self.loaded_files
                        );
                    }
                    FocusedPanel::SourceFiles => {
                        self.source_files_panel.toggle_selected(
                            &mut self.selected_extensions,
                            &mut self.selected_files,
                            &self.loaded_files
                        );
                    }
                    _ => {}
                }
            }
            _ => {
                match self.focused_panel {
                    FocusedPanel::SourcePath => {
                        self.source_path_panel.handle_input(key_event);
                    }
                    FocusedPanel::Filters => {
                        self.filters_panel.handle_input(key_event);
                    }
                    FocusedPanel::SourceFiles => {
                        self.source_files_panel.handle_input(key_event);
                    }
                    FocusedPanel::Output => {
                        self.output_panel.handle_input(key_event);
                    }
                    FocusedPanel::OutputFile => {
                        self.output_file_panel.handle_input(key_event);
                    }
                }
            }
        }
        let new_focused_panel = self.focused_panel;
        if old_focused_panel == FocusedPanel::SourcePath && new_focused_panel != FocusedPanel::SourcePath {
            if self.source_path_panel.value != self.prev_source_path {
                self.reload_files_needed = true;
                self.prev_source_path = self.source_path_panel.value.clone();
            }
        }
    }

    async fn handle_enter(&mut self) {
        match self.focused_panel {
            FocusedPanel::Output => {
                if self.output_panel.destination == OutputDestination::Clipboard {
                    if !self.processing {
                        self.merge_needed = true;
                    }
                } else {
                    self.focused_panel = self.focused_panel.next_panel(self);
                    self.set_cursor_to_end();
                }
            }
            FocusedPanel::OutputFile => {
                if !self.processing {
                    self.merge_needed = true;
                }
            }
            _ => {
                self.focused_panel = self.focused_panel.next_panel(self);
                self.set_cursor_to_end();
            }
        }
    }

    pub async fn reload_files_immediate(&mut self) {
        self.reload_files_needed = false;
        let path = self.source_path_panel.value.clone();
        let ts_result = create_text_source(&path).await;
        if let Ok(ts) = ts_result {
            self.text_source = Some(ts);
            if let Some(ts2) = &self.text_source {
                let index_res = ts2.get_file_index(&self.filter_config).await;
                match index_res {
                    Ok(files) => self.loaded_files = files,
                    Err(_) => self.loaded_files.clear(),
                }
            }
        } else {
            self.text_source = None;
            self.loaded_files.clear();
        }
        self.filters_panel.init_values(
            &self.loaded_files,
            &mut self.selected_extensions,
            &mut self.selected_files
        );
        self.source_files_panel.init_values(
            &self.loaded_files,
            &mut self.selected_files
        );
    }

    pub async fn merge_immediate(&mut self) {
        self.merge_needed = false;
        let mut files_map = std::collections::HashMap::new();
        for f in &self.loaded_files {
            if self.selected_files.contains(&f.path) {
                files_map.insert(f.path.clone(), f.clone());
            }
        }
        let output_file = self.output_file_panel.value.clone();
        let dest = self.output_panel.destination.clone();
        let contents = write_merged(&dest, &output_file, files_map, self).await;
        if let Ok(merged) = contents {
            if matches!(dest, OutputDestination::FileAndClipboard) {
                let _ = copy_clipboard(merged);
            }
        }
    }

    fn set_cursor_to_end(&mut self) {
        match self.focused_panel {
            FocusedPanel::SourcePath => {
                self.source_path_panel.cursor_pos = self.source_path_panel.value.len();
            }
            FocusedPanel::OutputFile => {
                self.output_file_panel.cursor_pos = self.output_file_panel.value.len();
            }
            _ => {}
        }
    }

    pub async fn reload_file_content(&mut self, sf: &SourceFile) -> Result<String, String> {
        if let Some(ts) = &self.text_source {
            ts.get_file_content(sf).await.map_err(|e| e.to_string())
        } else {
            Err("No text source available".to_string())
        }
    }
}