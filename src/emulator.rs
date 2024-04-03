use std::{fs, io};
use std::io::Error;
use std::ops::{Deref, Div};
use std::path::Path;
use log::Level;
use rand::Rng;
use itertools::Itertools;

#[derive(Debug)]
pub enum EmulationErr {
    UnknownOpcode(u16),
    StackOverflow,
    InvalidRegister,
    FileError
}

/// Emulator of Chip-8
pub struct Chip8Emu {
    opcode: u16,

    memory: Vec<u8>,
    registers: Vec<u8>,
    index_register: u16,
    program_counter: u16,

    gfx: Vec<u8>,

    delay_timer: u8,
    sound_timer: u8,

    stack: Vec<u16>,
    stack_pointer: u16,

    keys: Vec<bool>
}

impl Default for Chip8Emu {
    fn default() -> Self {
        Self {
            opcode: 0x0000,
            memory: vec![0x00; 4096],
            registers: vec![0x00; 16],
            index_register: 0x0000,
            program_counter: 0x0200,
            gfx: vec![0x00; 8 * 32],
            delay_timer: 0x00,
            sound_timer: 0x00,
            stack: vec![0x0000; 16],
            stack_pointer: 0x0000,
            keys: vec![false; 16],
        }
    }
}

impl Chip8Emu {
    pub fn new() -> Self { Self::default() }
    pub fn screen(&self) -> Vec<u8> { self.gfx.clone() }
    
    pub fn get_opcode(&self) -> u16 { self.opcode }

    pub fn get_opcodes(&self) -> Vec<u16> {
        let result: Vec<u16> = Vec::new();



        result
    }

    pub fn load_rom_from_file(&mut self, file_path: &str) -> Result<(),
        EmulationErr> {
        let file_contents = fs::read(file_path);

        match file_contents {
            Ok(mut bytes) => {
                let length = bytes.len();
                self.memory = vec![0x00; 512];
                self.memory.append(&mut bytes);
                self.memory.append(&mut vec![0x00; 4096 - length - 511]);
                log::log!(Level::Info, "ROM loaded from file {}", file_path);
                Ok(())
            }
            Err(_) => {
                Err(EmulationErr::FileError)
            }
        }

    }

    pub fn emulate_cycle(&mut self) -> Result<(), EmulationErr> {
        // Fetch opcode
        let first_byte = self.memory[self.program_counter as usize] as u16;
        let second_byte = self.memory[(self.program_counter + 1) as usize] as u16;
        self.opcode = (first_byte << 8) | second_byte;

        // Advance `program_counter`
        self.program_counter += 2;

        // Decode opcode
        let x = ((self.opcode & 0x0F00) >> 8) as usize;
        let y = ((self.opcode & 0x00F0) >> 4) as usize;
        let n: u8 = (self.opcode & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;
        let nnn: u16 = self.opcode & 0x0FFF;



        // Execute opcode
        match self.opcode {
            // 0x00E0 - Clear screen
            0x00E0 => {
                self.gfx = vec![0x00; 8 * 32];
                log::log!(Level::Info, "Clearing the screen");
            },

            // 0x1NNN - Jump to NNN
            0x1000..=0x1FFF => {
                self.program_counter = nnn;
                log::log!(Level::Info, "Set PC to 0x{:X}", nnn);
            },

            // 0x6XNN - Set register VX to NN
            0x6000..=0x6EFF => {
                self.registers[x] = nn;
                log::log!(Level::Info, "Set register V{:X} to {}", x, nn);
            },

            // 0x7XNN - Add NN to register VX
            0x7000..=0x7EFF => {
                self.registers[x] = self.registers[x].wrapping_add(nn);
                log::log!(Level::Info, "Added {} to register V{:X}", nn, x);
            }

            // 0xANNN - Set index register to NNN
            0xA000..=0xAFFF => {
                self.index_register = nnn;
                log::log!(Level::Info, "Set index register to 0x{:X}", nnn);
            }

            // 0xDXYN - Draw N bytes starting at memory address in index register at (VX, VY)
            0xD000..=0xDFFF => {
                let cx: u8 = self.registers[x] & 0x3F;
                let cy: u8 = self.registers[y] & 0x1F;
                self.registers[0xF] = 0x00;

                for row in 0..n as u16 {
                    let row_data: u8 = self.memory[(self.index_register + row) as usize];
                    let screen_byte_index = cy * 8 + cx.div(8);
                    let shift = cx % 8;
                    let initial_screen_state = self.gfx[screen_byte_index as usize];
                    self.gfx[screen_byte_index as usize] ^= row_data >> shift;

                    if (shift != 0) & (cx < 56) {
                        self.gfx[screen_byte_index as usize + 1] ^= row_data << (8 - shift);
                    }

                    if (initial_screen_state << shift) & row_data != 0 {
                        self.registers[0xF] = 0x01;
                    }

                    
                }

                log::log!(Level::Info, "Drawn to screen");
                for mut line in &self.gfx.clone().into_iter().chunks(8) {
                    let b1 = line.next().unwrap();
                    let b2 = line.next().unwrap();
                    let b3 = line.next().unwrap();
                    let b4 = line.next().unwrap();
                    let b5 = line.next().unwrap();
                    let b6 = line.next().unwrap();
                    let b7 = line.next().unwrap();
                    let b8 = line.next().unwrap();
                    log::log!(Level::Info, "{:8b} {:8b} {:8b} {:8b} {:8b} {:8b} {:8b} {:8b}", b1, b2,
                            b3, b4, b5, b6, b7, b8);
                }
            }

            _ => { return Err(EmulationErr::UnknownOpcode(self.opcode)) }
        }
        log::log!(Level::Info, "Executed opcode: 0x{:X}, registers: {:?}", self.opcode, self
            .registers);
        // Update timers

        Ok(())
    }
}