use std::fs;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListState};
use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

pub struct FileSelector {
    state: ListState,
    filenames: Vec<String>,
    selected_file: usize,
    is_focused: bool,
}

impl FileSelector {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            filenames: vec![],
            selected_file: 0,
            is_focused: false,
        }
    }
}

impl Component for FileSelector {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::MoveFileSelectorDown => { 
                self.selected_file = if self.selected_file != 0 {
                    self.selected_file - 1
                } else {
                    self.filenames.len() - 1
                };
            },
            
            Action::MoveFileSelectorUp => { 
                self.selected_file = if self.selected_file >= self.filenames.len() {
                    0
                } else {
                    self.selected_file + 1
                };
            },
            
            Action::SelectFile => {
                self.is_focused = false;
                return Ok(Some(Action::LoadFile(self.filenames[self.selected_file].clone())))
            }
            
            Action::FocusFileSelector => self.is_focused = true,
            
            _ => {}
        }
        
        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
        let chunks_h = Layout::horizontal(
            vec![
                Constraint::Length(130),
                Constraint::Fill(1),
                Constraint::Length(16),
            ]
        ).split(area);

        let chunks_v = Layout::vertical(
            vec![
                Constraint::Fill(1),
                Constraint::Length(3),
            ]
        ).split(chunks_h[1]);
        
        self.filenames.clear();
        fs::read_dir("./scripts/").unwrap().for_each(
            |x| {
                if let Ok(entry) = x {
                    if entry.metadata().unwrap().is_file() {
                        self.filenames.push(
                            entry.file_name().into_string().unwrap()
                        )
                    }
                }
            }
        );

        let list = List::new(self.filenames.clone())
            .block(Block::default().title("Scripts").borders(Borders::ALL).border_style(
                Style::default().fg(if self.is_focused { Color::Cyan } else { Color::White })
            ))
            .highlight_symbol(">>")
            .highlight_style(Style::default().fg(Color::LightBlue));
        
        self.state.select(Some(self.selected_file));

        f.render_stateful_widget(list, chunks_v[0], &mut self.state);
        
        Ok(())
    }
}