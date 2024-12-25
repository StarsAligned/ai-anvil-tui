use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Rect, Alignment},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::output::clipboard::{copy_clipboard, get_clipboard_content};

pub struct SourcePathPanel {
    pub value: String,
    pub cursor_pos: usize,
}

impl SourcePathPanel {
    pub fn new(value: String) -> Self {
        let end = value.len();
        Self {
            value,
            cursor_pos: end,
        }
    }

    pub fn draw(&self, f: &mut ratatui::Frame, area: Rect, focused: bool) {
        let block_style = if focused {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default()
        };
        let block = Block::default()
            .title("Source (Directory path or Github URL)")
            .borders(Borders::ALL)
            .style(block_style);
        let mut spans = Vec::new();
        if self.cursor_pos <= self.value.len() {
            let first_text = &self.value[..self.cursor_pos];
            if !first_text.is_empty() {
                spans.push(Span::styled(first_text, Style::default().fg(Color::White)));
            }
            if focused {
                spans.push(Span::styled("█", Style::default().fg(Color::DarkGray)));
            }
            let second_text = &self.value[self.cursor_pos..];
            if !second_text.is_empty() {
                spans.push(Span::styled(second_text, Style::default().fg(Color::White)));
            }
        } else {
            spans.push(Span::styled(&self.value, Style::default().fg(Color::White)));
        }
        let line = Line::from(spans);
        let paragraph = Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Left);
        f.render_widget(paragraph, area);
    }

    pub fn handle_input(&mut self, key_event: KeyEvent) {
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
        match key_event.code {
            KeyCode::Char('c') if ctrl => {
                let _ = copy_clipboard(self.value.clone());
            }
            KeyCode::Char('v') if ctrl => {
                if let Ok(contents) = get_clipboard_content() {
                    self.value.insert_str(self.cursor_pos, &contents);
                    self.cursor_pos += contents.len();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                self.value.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.value.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.value.len() {
                    self.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }
}