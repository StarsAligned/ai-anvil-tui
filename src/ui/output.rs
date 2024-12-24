use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
};

#[derive(Clone, PartialEq)]
pub enum OutputDestination {
    FileAndClipboard,
    File,
    Clipboard,
}

pub struct OutputPanel {
    pub items: Vec<OutputDestination>,
    pub selected: usize,
    pub destination: OutputDestination,
}

impl OutputPanel {
    pub fn new() -> Self {
        let items = vec![
            OutputDestination::FileAndClipboard,
            OutputDestination::File,
            OutputDestination::Clipboard,
        ];
        Self {
            items,
            selected: 0,
            destination: OutputDestination::FileAndClipboard,
        }
    }
    pub fn draw(&mut self, f: &mut ratatui::Frame, area: Rect, focused: bool) {
        let block_style = if focused {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Output")
            .style(block_style);
        let lines: Vec<Line> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let text = match o {
                    OutputDestination::FileAndClipboard => "File + Clipboard",
                    OutputDestination::File => "File",
                    OutputDestination::Clipboard => "Clipboard",
                };
                let selected = i == self.selected;
                let icon = if selected { "[x]" } else { "[ ]" };
                let style = if focused {
                    if selected {
                        Style::default().fg(Color::LightBlue)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }
                } else {
                    if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }
                };
                Line::styled(format!("{} {}", icon, text), style)
            })
            .collect();
        let divider_span = Span::styled("|", Style::default().fg(Color::DarkGray));
        let tabs = Tabs::new(lines)
            .block(block)
            .select(self.selected)
            .divider(divider_span)
            .highlight_style(Style::default().fg(Color::White));
        f.render_widget(tabs, area);
    }
    pub fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Right => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                self.destination = self.items[self.selected].clone();
            }
            _ => {}
        }
        self.destination = self.items[self.selected].clone();
    }
}