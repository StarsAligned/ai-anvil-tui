use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, ListItem},
};
use std::collections::{HashMap, HashSet};
use crate::input::SourceFile;

pub enum TokenStatus {
    NotCounted,
    Counting,
    Done(usize),
    Error,
}

pub struct SourceFilesPanel {
    pub items: Vec<String>,
    pub cursor: usize,
    pub offset: usize,
    pub file_token_status: HashMap<String, TokenStatus>,
    pub panel_title: String,
}

impl SourceFilesPanel {
    pub fn new() -> Self {
        Self {
            items: vec![],
            cursor: 0,
            offset: 0,
            file_token_status: HashMap::new(),
            panel_title: "Files".to_string(),
        }
    }

    pub fn init_values(&mut self, files: &Vec<SourceFile>, selected_files: &mut HashSet<String>) {
        let mut paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        paths.sort();
        self.items = paths;
        for f in files {
            selected_files.insert(f.path.clone());
            self.file_token_status.insert(f.path.clone(), TokenStatus::NotCounted);
        }
        self.cursor = 0;
        self.offset = 0;
        self.panel_title = "Files".to_string();
    }

    pub fn draw(&self, f: &mut ratatui::Frame, area: Rect, focused: bool, selected_files: &HashSet<String>) {
        let block_style = if focused {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title(self.panel_title.as_str())
            .borders(Borders::ALL)
            .style(block_style);

        let max_path_len = self.items.iter().map(|p| p.len()).max().unwrap_or(0);

        let mut status_map = HashMap::new();
        for path in &self.items {
            let status_str = self.get_status_string(path);
            status_map.insert(path, status_str);
        }
        let max_status_len = status_map.values().map(|s| s.len()).max().unwrap_or(0);

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
                } else if is_selected {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let padded_path = format!("{:width$}", it, width = max_path_len);
                let status_str = status_map[it].clone();
                let right_aligned_status = format!("{:>width$}", status_str, width = max_status_len);

                let line = format!("{}{} {}  {}", prefix, icon, padded_path, right_aligned_status);
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

    pub fn set_counting(&mut self, path: &str) {
        self.file_token_status.insert(path.to_string(), TokenStatus::Counting);
    }

    pub fn set_count_result(&mut self, path: &str, result: Result<usize, String>) {
        match result {
            Ok(n) => {
                self.file_token_status.insert(path.to_string(), TokenStatus::Done(n));
            }
            Err(_) => {
                self.file_token_status.insert(path.to_string(), TokenStatus::Error);
            }
        }
    }

    pub fn update_title_counting(&mut self) {
        self.panel_title = "Files (counting tokens)".to_string();
    }

    pub fn update_title_sum(&mut self, selected_files: &HashSet<String>) {
        if let Some(sum) = self.maybe_compute_total_tokens(selected_files) {
            self.panel_title = format!("Files ({} tokens)", format_number(sum));
        }
    }

    fn maybe_compute_total_tokens(&self, selected_files: &HashSet<String>) -> Option<usize> {
        for path in selected_files {
            match self.file_token_status.get(path) {
                Some(TokenStatus::NotCounted) | Some(TokenStatus::Counting) => return None,
                _ => {}
            }
        }
        let mut total = 0;
        for path in selected_files {
            if let Some(TokenStatus::Done(n)) = self.file_token_status.get(path) {
                total += n;
            }
        }
        Some(total)
    }

    fn get_status_string(&self, path: &str) -> String {
        match self.file_token_status.get(path) {
            Some(TokenStatus::Counting) => "...".to_owned(),
            Some(TokenStatus::Done(n)) => format_token_count(*n),
            Some(TokenStatus::Error) => "Error".to_owned(),
            _ => "".to_owned(),
        }
    }
}

fn format_token_count(n: usize) -> String {
    let s = format_number(n);
    format!("{} tokens", s)
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, ' ');
        }
        result.insert(0, c);
        count += 1;
    }
    result
}