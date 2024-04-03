use std::ops::Div;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Text;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::UnboundedSender;
use tracing_subscriber::fmt::format;
use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

pub struct Screen {
    screen_data: Vec<String>,
    is_running: bool,
}

impl Screen {
    pub fn new() -> Self { Self {screen_data: Vec::new(), is_running: false } }
}

impl Component for Screen {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Redraw(data) => {
                self.screen_data = Vec::new();
                for row in 0..32 {
                    let mut row_data = String::new();
                    for column in 0..8 {
                        let mut byte = data[row * 8 + column];
                        let mut byte_data = String::new();
                        let leading_space =  String::from(' ').repeat(
                            (byte.leading_zeros() * 2) as usize
                        );
                        while byte > 0 {
                            byte_data.insert(
                                0,
                                match byte % 2 {
                                    0 => { ' ' }
                                    1 => { '█' }
                                    _ => unreachable!()
                                }
                            );
                            byte_data.insert(
                                0,
                                match byte % 2 {
                                    0 => { ' ' }
                                    1 => { '█' }
                                    _ => unreachable!()
                                }
                            );
                            byte = byte.div(2);
                        }
                        byte_data = leading_space + byte_data.as_str();
                        row_data += byte_data.as_str();
                    }
                    self.screen_data.push(row_data);
                }
            }
            Action::StartEmulation => { self.is_running = true }
            Action::StopEmulation => { self.is_running = false }
            
            _ => {}
        }

        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        let chunks_h = Layout::horizontal(
            vec![
                Constraint::Length(130),
                Constraint::Min(3),
            ]
        ).split(area);

        let chunks_v = Layout::vertical(
            vec![
                Constraint::Length(34),
                Constraint::Min(3),
            ]
        ).split(chunks_h[0]);

        let mut text = Vec::<Line>::new();
        
        self.screen_data.iter().for_each(
            |line| {
                text.push(Line::from(line.clone()))
            }
        );

        let screen = Paragraph::new(text)
            .block(
                Block::default().title("Screen").borders(Borders::ALL).border_style(
                    Style::default().fg(
                        if self.is_running { Color::LightCyan } else { Color::White }
                    )
                )
            );
        
        f.render_widget(screen, chunks_v[0]);

        Ok(())
    }
}