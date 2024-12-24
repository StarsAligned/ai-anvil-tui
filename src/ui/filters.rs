use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, ListItem},
};
use std::collections::{BTreeSet, HashSet};
use crate::input::SourceFile;

pub struct FiltersPanel {
    pub items: Vec<String>,
    pub cursor: usize,
    pub offset: usize,
}

impl FiltersPanel {
    pub fn new() -> Self {
        Self {
            items: vec![],
            cursor: 0,
            offset: 0,
        }
    }
    pub fn init_values(
        &mut self,
        files: &Vec<SourceFile>,
        selected_exts: &mut HashSet<String>,
        selected_files: &mut HashSet<String>,
    ) {
        let mut exts = BTreeSet::new();
        for f in files {
            if let Some(ext) = f.path.split('.').last() {
                exts.insert(ext.to_string());
            }
        }
        let mut new_items = vec!["*".to_string()];
        for e in exts {
            new_items.push(e);
        }
        self.items = new_items;
        for it in &self.items {
            selected_exts.insert(it.clone());
        }
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
        selected_exts: &HashSet<String>,
    ) {
        let block_style = if focused {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title("Filters")
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
                let is_selected = selected_exts.contains(it);
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
    pub fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
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
        let selected_item = &self.items[self.cursor];
        let is_already_selected = selected_exts.contains(selected_item);
        if selected_item == "*" {
            if is_already_selected {
                selected_exts.remove("*");
                for i in &self.items {
                    selected_exts.remove(i);
                }
                selected_files.clear();
            } else {
                selected_exts.insert("*".to_string());
                for i in &self.items {
                    selected_exts.insert(i.clone());
                }
                selected_files.clear();
                for f in all_files {
                    selected_files.insert(f.path.clone());
                }
            }
        } else {
            if is_already_selected {
                selected_exts.remove(selected_item);
                for f in all_files {
                    if let Some(ext) = f.path.split('.').last() {
                        if ext == selected_item {
                            selected_files.remove(&f.path);
                        }
                    }
                }
                selected_exts.remove("*");
            } else {
                selected_exts.insert(selected_item.clone());
                for f in all_files {
                    if let Some(ext) = f.path.split('.').last() {
                        if ext == selected_item {
                            selected_files.insert(f.path.clone());
                        }
                    }
                }
                let all_included = self.items.iter().all(|it| selected_exts.contains(it));
                if all_included {
                    selected_exts.insert("*".to_string());
                }
            }
        }
    }
}