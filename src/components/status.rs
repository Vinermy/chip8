use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

pub struct StatusBar {
    opcode: u16,
}

impl StatusBar {
    pub fn new() -> Self { Self {opcode: 0x0000} }
}

impl Component for StatusBar {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if let Action::UpdateOpcode(opcode) = action {
            self.opcode = opcode;
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        let chunks_v = Layout::vertical(
            vec![
                Constraint::Fill(1),
                Constraint::Length(3),
            ]
        ).split(area);

        let status = Paragraph::new(format!("Current opcode: 0x{:X}", self.opcode))
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status, chunks_v[1]);

        Ok(())
    }
}