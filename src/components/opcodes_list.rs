use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListDirection, ListState};
use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

#[derive(Default)]
pub struct  OpcodesList {
    state: ListState,
    opcodes: Vec<u16>,
    current_opcode: u16,
}

impl OpcodesList {
    pub fn new() -> Self { Self::default() }
}

impl Component for OpcodesList {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::LoadOpcodesList(data) => {
                self.opcodes = data;
            }
            Action::SelectOpcode(i) => {
                self.state.select(Some(i as usize))
            }

            _ => {}
        }

        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        let v_chunks = Layout::vertical(
            vec![
                Constraint::Fill(1),
                Constraint::Length(3),
            ]
        ).split(area);

        let h_chunks = Layout::horizontal(
            vec![
                Constraint::Fill(1),
                Constraint::Length(16),
            ]
        ).split(v_chunks[0]);

        let list = List::new(self.opcodes.iter().map(|x| {format!("0x{:0>4X}", x)}))
            .block(Block::default().title("Program").borders(Borders::ALL))
            .style(Style::default())
            .highlight_style(Style::default().fg(Color::LightBlue))
            .highlight_symbol(">>")
            .direction(ListDirection::TopToBottom);

        f.render_stateful_widget(list, h_chunks[1], &mut self.state);

        Ok(())
    }
}