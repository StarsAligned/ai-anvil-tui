use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, ListItem},
};
use crate::input::SourceFile;
use std::collections::HashSet;

pub struct SourceFilesPanel {
    pub items: Vec<String>,
    pub cursor: usize,
    pub offset: usize,
}

impl SourceFilesPanel {
    pub fn new() -> Self {
        Self {
            items: vec![],
            cursor: 0,
            offset: 0,
        }
    }

    pub fn init_values(&mut self, files: &Vec<SourceFile>, selected_files: &mut HashSet<String>) {
        let mut paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        paths.sort();
        self.items = paths;
        for f in files {
            selected_files.insert(f.path.clone());
        }
        self.cursor = 0;
        self.offset = 0;
    }

    pub fn draw(
        &self,
        f: &mut ratatui::Frame,
        area: Rect,
        focused: bool,
        selected_files: &HashSet<String>,
    ) {
        let block_style = if focused {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title("Files")
            .borders(Borders::ALL)
            .style(block_style);
        let visible_count = area.height.saturating_sub(2) as usize;
        let end = (self.offset + visible_count).min(self.items.len());
        let slice = &self.items[self.offset..end];
        let list_items: Vec<ListItem> = slice
            .iter()
            .enumerate()
            .map(|(idx_in_slice, it)| {
                let i = self.offset + idx_in_slice;
                let is_selected = selected_files.contains(it);
                let icon = if is_selected { "[x]" } else { "[ ]" };
                let prefix = if i == self.cursor { "> " } else { "  " };
                let item_style = if focused && i == self.cursor {
                    Style::default().fg(Color::LightBlue)
                } else {
                    if is_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }
                };
                let line = format!("{}{} {}", prefix, icon, it);
                ListItem::new(line).style(item_style)
            })
            .collect();
        let list = ratatui::widgets::List::new(list_items).block(block);
        f.render_widget(list, area);
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    if self.cursor < self.offset {
                        self.offset = self.cursor;
                    }
                }
            }
            KeyCode::Down => {
                if self.cursor + 1 < self.items.len() {
                    self.cursor += 1;
                    let visible_count = 10;
                    if self.cursor >= self.offset + visible_count {
                        self.offset = self.cursor + 1 - visible_count;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn toggle_selected(
        &mut self,
        selected_exts: &mut HashSet<String>,
        selected_files: &mut HashSet<String>,
        all_files: &Vec<SourceFile>,
    ) {
        if self.items.is_empty() {
            return;
        }
        let current_file = &self.items[self.cursor];
        if selected_files.contains(current_file) {
            selected_files.remove(current_file);
            for f in all_files {
                if f.path == *current_file {
                    if let Some(ext) = current_file.split('.').last() {
                        if selected_exts.contains(ext) {
                            selected_exts.remove(ext);
                        }
                    }
                }
            }
            selected_exts.remove("*");
        } else {
            selected_files.insert(current_file.clone());
            if let Some(ext) = current_file.split('.').last() {
                let mut all_same_ext = true;
                for f in all_files {
                    if let Some(e2) = f.path.split('.').last() {
                        if e2 == ext && !selected_files.contains(&f.path) {
                            all_same_ext = false;
                            break;
                        }
                    }
                }
                if all_same_ext {
                    selected_exts.insert(ext.to_string());
                }
            }
        }
    }
}