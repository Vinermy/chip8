use std::collections::HashMap;
use std::path::Components;
use std::time::Instant;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing_subscriber::fmt::format;

use crate::{
  action::Action,
  components::{Component, screen::Screen},
  config::Config,
  mode::Mode,
  tui,
};
use crate::components::file_selector::FileSelector;
use crate::components::opcodes_list::OpcodesList;
use crate::components::status::StatusBar;
use crate::emulator::Chip8Emu;

const KEYBOARD: [KeyCode; 16] = [
  KeyCode::Char('1'), KeyCode::Char('2'), KeyCode::Char('3'), KeyCode::Char('4'),
  KeyCode::Char('q'), KeyCode::Char('w'), KeyCode::Char('e'), KeyCode::Char('r'),
  KeyCode::Char('a'), KeyCode::Char('s'), KeyCode::Char('d'), KeyCode::Char('f'),
  KeyCode::Char('z'), KeyCode::Char('x'), KeyCode::Char('c'), KeyCode::Char('v'),
];

fn get_key_from_char(c: &char) -> u8 {
  match c {
    '1' => 1,
    '2' => 2,
    '3' => 3,
    '4' => 12,
    'q' => 4,
    'w' => 5,
    'e' => 6,
    'r' => 13,
    'a' => 7,
    's' => 8,
    'd' => 9,
    'f' => 14,
    'z' => 10,
    'x' => 0,
    'c' => 11,
    'v' => 15,
    _ => u8::MAX,
  }
}

pub struct App {
  pub config: Config,
  pub tick_rate: f64,
  pub frame_rate: f64,
  pub components: Vec<Box<dyn Component>>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub last_tick_key_events: Vec<KeyEvent>,
  pub emulator: Chip8Emu,
  pub running: bool,
  last_timer_tick: Option<Instant>,
  emu_ready: bool,
  script_filename: String,
}

impl App {
  pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
    let config = Config::new()?;
    let screen = Screen::new();
    let status = StatusBar::new();
    let opcode_list = OpcodesList::new();
    let file_selector = FileSelector::new();
    let mode = Mode::Home;
    Ok(Self {
      tick_rate,
      frame_rate,
      components: vec![Box::new(screen), Box::new(status), Box::new(opcode_list), Box::new(file_selector)],
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      last_tick_key_events: Vec::new(),
      emulator: Chip8Emu::new(),
      running: false,
      last_timer_tick: None,
      emu_ready: false,
      script_filename: "".to_string(),
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let mut tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
    // tui.mouse(true);
    tui.enter()?;

    for component in self.components.iter_mut() {
      component.register_action_handler(action_tx.clone())?;
    }

    for component in self.components.iter_mut() {
      component.register_config_handler(self.config.clone())?;
    }

    for component in self.components.iter_mut() {
      component.init(tui.size()?)?;
    }



    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            if let KeyCode::Char(keycode) = key.code {
              if KEYBOARD.contains(&key.code) {
                log::info!("CAPTURED KEY PRESS");
                let r = self.emulator.press(
                  &get_key_from_char(&keycode)
                );
                if let Err(err) = r {
                  log::error!("Error while capturing key press: {}", String::from(err))
                }
              }
            }

            if let Some(keymap) = self.config.keybindings.get(&self.mode) {
              if let Some(action) = keymap.get(&vec![key]) {
                log::info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
              } else {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                  log::info!("Got action: {action:?}");
                  action_tx.send(action.clone())?;
                }
              }
            };
          },
          _ => {},
        }
        for component in self.components.iter_mut() {
          if let Some(action) = component.handle_events(Some(e.clone()))? {
            action_tx.send(action)?;
          }
        }
      }

      while let Ok(action) = action_rx.try_recv() {
        if action != Action::Tick && action != Action::Render {
          log::debug!("{action:?}");
        }
        match action {
          Action::Tick => {
            self.last_tick_key_events.drain(..);
            if self.running {
              action_tx.send(Action::UpdateOpcode(self.emulator.get_opcode())).expect("Can send an action");
              if let Err(emu_err) = self.emulator.emulate_cycle() {
                action_tx.send(Action::Error(emu_err.clone().into())).expect("Can send an action");
                log::error!("{}", String::from(emu_err));
              }
              action_tx.send(Action::SelectOpcode(self.emulator.get_program_counter() - 512))
                  .expect("Can send an action");
              
              if let Some(last_tick) = self.last_timer_tick {
                if Instant::now().duration_since(last_tick).as_millis() > 16 {
                  self.last_timer_tick = Some(Instant::now());
                  
                  action_tx.send(Action::Redraw(self.emulator.screen()))
                      .expect("Can send an action");
                  self.emulator.update_delay_timer();
                  if self.emulator.update_sound_timer() {
                    // BEEP!!!
                  }
                }
              }
            }
          },
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              for component in self.components.iter_mut() {
                let r = component.draw(f, f.size());
                if let Err(e) = r {
                  action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
                }
              }
            })?;
          },
          Action::StartEmulation => { self.running = true; self.last_timer_tick = Some(Instant::now()) },
          Action::StopEmulation => { self.running = false; self.last_timer_tick = None },
          Action::FocusFileSelector => { self.mode = Mode::SelectingFile },
          Action::LoadFile(ref filename) => {
            self.mode = Mode::Home;
            self.emu_ready = true;
            self.script_filename = filename.clone();

            self.emulator.load_rom_from_file(format!("./scripts/{}", self.script_filename).as_str()).expect("Can read file");
            action_tx.send(Action::LoadOpcodesList(self.emulator.get_opcodes()));
            action_tx.send(Action::SelectOpcode(0));
          }
          _ => {},
        }
        for component in self.components.iter_mut() {
          if let Some(action) = component.update(action.clone())? {
            action_tx.send(action)?
          };
        }
      }
      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;
      } else if self.should_quit {
        tui.stop()?;
        break;
      }
    }
    tui.exit()?;
    Ok(())
  }
}
