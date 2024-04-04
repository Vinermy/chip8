use std::fs;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, List, ListState};
use crate::components::Component;
use crate::tui::Frame;

pub struct FileSelector {
    state: ListState,
    filenames: Vec<u16>,
    selected_file: usize,
}

impl FileSelector {
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            filenames: vec![],
            selected_file: 0,
        }
    }
}

impl Component for FileSelector {
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

        let mut filenames: Vec<String> = Vec::new();

        fs::read_dir("./scripts/").unwrap().for_each(
            |x| {
                if let Ok(entry) = x {
                    if entry.metadata().unwrap().is_file() {
                        filenames.push(
                            entry.file_name().into_string().unwrap()
                        )
                    }
                }
            }
        );

        let list = List::new(filenames)
            .block(Block::default().title("Scripts"));

        f.render_stateful_widget(list, chunks_v[0], &mut self.state);
        
        Ok(())
    }
}