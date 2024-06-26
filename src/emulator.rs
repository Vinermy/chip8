use std::{fs, io};
use std::io::Error;
use std::ops::{Deref, Div};
use std::path::Path;
use clap::builder::Str;
use log::Level;
use rand::Rng;
use itertools::{Itertools, Tuples};
use itertools::traits::HomogeneousTuple;

#[derive(Debug, Clone)]
pub enum EmulationErr {
    UnknownOpcode(u16),
    StackOverflow,
    InvalidValueInRegister(u8, u8),
    FileError(String),
    NoSubroutineToExit,
    InvalidKeycode,
    ProgramExited,
    InvalidRegisterReference,
}

impl From<EmulationErr> for String {
    fn from(err: EmulationErr) -> String {
        match err {
            EmulationErr::UnknownOpcode(opcode) => {
                format!("Unknown/unimplemented opcode encountered: 0x{:0>4X}", opcode)
            }
            EmulationErr::StackOverflow => {
                "Stack overflow".to_string()
            }
            EmulationErr::InvalidValueInRegister(register, value) => {
                format!("Register V{:X} contains invalid value 0x{:0>4X}", register, value)
            }
            EmulationErr::FileError(filename) => {
                format!("Can't read file {}", filename)
            }
            EmulationErr::NoSubroutineToExit => {
                "No subroutine to exit".to_string()
            }
            EmulationErr::InvalidKeycode => {
                "Invalid keycode supplied".to_string()
            }
            EmulationErr::ProgramExited => {
                "Program exited".to_string()
            }
            EmulationErr::InvalidRegisterReference => {
                "Invalid register reference supplied".to_string()
            }
        }
    }
}


fn font() -> Vec<u8> {
    vec![
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ]
}

#[derive(Default)]
struct Quirks {
    superchip_opcodes: bool, // Enables opcodes that were *added* in Superchip
    superchip_shift: bool, // Enables the new behaviour of 0x8XY6 and 0x8XYE from Superchip
    superchip_offset_jump: bool, // Enables the 0xBNNN behaviour from Superchip
    superchip_memory: bool, // Enables the 0xFX55 and 0xFX65 behaviour from Superchip
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

    keys: Vec<bool>,
    quirks: Quirks,

    // SUPERCHIP related features
    rpl: Vec<u8>,
    is_hi_res_mode: bool
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
            quirks: Quirks::default(),
            rpl: vec![0x00; 8],
            is_hi_res_mode: false,
        }
    }
}

impl Chip8Emu {
    pub fn new() -> Self { Self::default() }
    pub fn screen(&self) -> Vec<u8> { self.gfx.clone() }
    
    fn reset(&mut self) {
        self.opcode = 0x0000;
        self.memory = vec![0x00; 4096];
        self.registers = vec![0x00; 16];
        self.index_register = 0x0000;
        self.program_counter = 0x0200;
        self.gfx = vec![0x00; 8 * 32];
        self.delay_timer = 0x00;
        self.sound_timer = 0x00;
        self.stack = vec![0x0000; 16];
        self.stack_pointer = 0x0000;
        self.keys = vec![false; 16];
        self.is_hi_res_mode = false;
    }
    
    pub fn get_opcode(&self) -> u16 { self.opcode }
    pub fn get_program_counter(&self) -> u16 { self.program_counter }

    pub fn get_opcodes(&self) -> Vec<u16> {
        let mut result: Vec<u16> = Vec::new();

        for (first_byte, second_byte) in self.memory[512..].iter().tuples() {
            let opcode = (*first_byte as u16) << 8 | (*second_byte as u16);
            result.push(opcode);
        }

        result
    }

    pub fn load_rom_from_file(&mut self, file_path: &str) -> Result<(),
        EmulationErr> {
        let file_contents = fs::read(file_path);

        match file_contents {
            Ok(mut bytes) => {
                self.reset();
                let length = bytes.len();
                self.memory = Vec::new();
                self.memory.append(&mut vec![0x00; 80]);
                self.memory.append(&mut font());
                self.memory.append(&mut vec![0x00; 512-160]);
                self.memory.append(&mut bytes);
                self.memory.append(&mut vec![0x00; 4096 - length - 511]);
                log::log!(Level::Info, "ROM loaded from file {}", file_path);
                Ok(())
            }
            Err(_) => {
                Err(EmulationErr::FileError(file_path.to_string()))
            }
        }

    }
    
    pub fn update_delay_timer(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }
    
    pub fn update_sound_timer(&mut self) -> bool {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            return true
        }
        false
    }

    pub fn press(&mut self, key: &u8) -> Result<(), EmulationErr> {
        if (0..=15).contains(key) {
            self.keys[*key as usize] = true;
        } else {
            return Err(EmulationErr::InvalidKeycode)
        }
        log::info!("Key press registered: {key}");
        Ok(())
    }

    pub fn release(&mut self, key: &u8) -> Result<(), EmulationErr> {
        if (0..=15).contains(key) {
            self.keys[*key as usize] = false;
        } else {
            return Err(EmulationErr::InvalidKeycode)
        }
        log::info!("Key release registered: {key}");
        Ok(())
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

            // 0x00EE - Exit from subroutine
            0x00EE => {
                self.program_counter = self.stack[self.stack_pointer as usize];
                log::info!("Exiting from subroutine to 0x{:0>3X}", self.program_counter);

                if self.stack_pointer == 0 {
                    return Err(EmulationErr::NoSubroutineToExit);
                } else {
                    self.stack_pointer -= 1;
                }
            },

            // 0x1NNN - Jump to NNN
            0x1000..=0x1FFF => {
                self.program_counter = nnn;
                log::log!(Level::Info, "Set PC to 0x{:0>3X}", nnn);
            },

            // 0x2NNN - Start subroutine from address NNN
            0x2000..=0x2FFF => {
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.program_counter = nnn;
                log::info!("Entered subroutine at 0x{:0>3X}", nnn)
            },

            // 0x3XNN - Skip one instruction if the value in VX is equal to NN
            0x3000..=0x3FFF => {
                if self.registers[x] == nn {
                    log::info!("Skipped instruction at 0x{:0>3X}", self.program_counter);
                    self.program_counter += 2;
                }
            },

            // 0x4XNN - Skip one instruction if the value in VX is not equal to NN
            0x4000..=0x4FFF => {
                if self.registers[x] != nn {
                    log::info!("Skipped instruction at 0x{:0>3X}", self.program_counter);
                    self.program_counter += 2;
                }
            },

            // 0x5XY0 - Skip one instruction if the value in VX is equal to value in VY
            0x5000..=0x5FF0 => {
                if self.registers[x] == self.registers[y] {
                    log::info!("Skipped instruction at 0x{:0>3X}", self.program_counter);
                    self.program_counter += 2;
                }
            },

            // 0x6XNN - Set register VX to NN
            0x6000..=0x6FFF => {
                self.registers[x] = nn;
                log::log!(Level::Info, "Set register V{:X} to {}", x, nn);
            },

            // 0x7XNN - Add NN to register VX
            0x7000..=0x7FFF => {
                self.registers[x] = self.registers[x].wrapping_add(nn);
                log::log!(Level::Info, "Added {} to register V{:X}", nn, x);
            },

            // 0x8XYN - Logical and arithmetic instructions
            0x8000..=0x8FFF => {
                match n {

                    // VX is set to the value of VY
                    0 => {
                        self.registers[x] = self.registers[y];
                        log::info!("Set the value of register {x} to the value of the register {y}")
                    },

                    // VX is set to the bitwise (OR) of VX and VY. VY is not affected.
                    1 => {
                        self.registers[x] |= self.registers[y];
                        log::info!("Set the register {x} to the bitwise OR of register {x} and register {y}")
                    },

                    // VX is set to the bitwise (AND) of VX and VY. VY is not affected.
                    2 => {
                        self.registers[x] &= self.registers[y];
                        log::info!("Set the register {x} to the bitwise AND of register {x} and register {y}")
                    },

                    // VX is set to the bitwise (XOR) of VX and VY. VY is not affected.
                    3 => {
                        self.registers[x] ^= self.registers[y];
                        log::info!("Set the register {x} to the bitwise XOR or register {x} and register {y}")
                    },

                    // VX is set to the value of VX plus the value of VY. VY is not affected.
                    4 => {
                        let (result, is_overflow) = self.registers[x]
                            .overflowing_add(self.registers[y]);
                        self.registers[x] = result;
                        self.registers[15] = is_overflow as u8;
                        log::info!("Set the register {x} to the sum of register {x} and register {y}")
                    },

                    // VX is set to the result of VX - VY
                    5 => {
                        let (result, is_overflow) = self.registers[x]
                            .overflowing_sub(self.registers[y]);
                        self.registers[x] = result;
                        self.registers[15] = 1 - (is_overflow as u8);
                        log::info!("Set the register {x} to the result of subtracting register {y} from register {x}")
                    }

                    // Sets VX equal to VY and shifts it one bit to the right. VF is set to the
                    // shifted out bit
                    6 => {
                        self.registers[x] = self.registers[y];
                        let shifted_out = self.registers[x] % 2;
                        self.registers[x] >>= 1;
                        self.registers[15] = shifted_out;
                        log::info!("Set the register {x} to the register {y} shifted one bit to the right")
                    },

                    // VX is set to the result of VY - VX
                    7 => {
                        let (result, is_overflow) = self.registers[y]
                            .overflowing_sub(self.registers[x]);
                        self.registers[x] = result;
                        self.registers[15] = 1 - (is_overflow as u8);
                        log::info!("Set the register {x} to the result of subtracting register {x} from register {y}")
                    },

                    // Sets VX equal to VY and shifts it one bit to the left. VF is set to the
                    // shifted out bit
                    0xE => {
                        self.registers[x] = self.registers[y];
                        let shifted_out = (self.registers[x] >= 128) as u8;
                        self.registers[x] <<= 1;
                        self.registers[15] = shifted_out;
                        log::info!("Set the register {x} to the register {y} shifted one bit to the left")
                    },

                    _ => {
                        return Err(EmulationErr::UnknownOpcode(self.opcode))
                    }
                }
            },

            // 0x5XY0 - Skip one instruction if the value in VX is not equal to value in VY
            0x9000..=0x9FF0 => {
                if self.registers[x] != self.registers[y] {
                    self.program_counter += 2;
                    log::info!("Skipped instruction at 0x{:0>3X}", self.program_counter);
                }
            },

            // 0xANNN - Set index register to NNN
            0xA000..=0xAFFF => {
                self.index_register = nnn;
                log::log!(Level::Info, "Set index register to 0x{:0>3X}", nnn);
            },

            // 0xBNNN - Jump with offset of NNN
            0xB000..=0xBFFF => {
                self.program_counter = nnn + self.registers[0] as u16;
                log::info!("Jumped to the 0x{:0>3X}", self.program_counter)
            },

            // 0xCXNN - Put random value with mask NN into VX
            0xC000..=0xCFFF => {
                let mut rng = rand::thread_rng();
                self.registers[x] = rng.gen_range(0..=255) & nn;
                log::info!("Set the register {x} to the random value of {}", self.registers[x])
            }

            // 0xDXYN - Draw N bytes starting at memory address in index register at (VX, VY)
            0xD000..=0xDFFF => {
                let cx: u8 = self.registers[x] & 0x3F;
                let cy: u8 = self.registers[y] & 0x1F;
                self.registers[0xF] = 0x00;

                for row in 0..n as u16 {
                    let row_data: u8 = self.memory[(self.index_register + row) as usize];
                    let screen_byte_index = cy * 8 + cx.div(8) + (row * 8) as u8;
                    let shift = cx % 8;
                    let initial_screen_state = self.gfx[screen_byte_index as usize];
                    self.gfx[screen_byte_index as usize] ^= row_data >> shift;
                    log::log!(Level::Info, "Drawn at {}: {:0>8b} -> {:0>8b}",
                        screen_byte_index,
                        initial_screen_state,
                        self.gfx[screen_byte_index as usize]);

                    if (shift != 0) & (cx < 56) {
                        self.gfx[screen_byte_index as usize + 1] ^= row_data << (8 - shift);
                    }

                    if (initial_screen_state << shift) & row_data != 0 {
                        self.registers[0xF] = 0x01;
                    }


                }

                log::log!(Level::Info, "Drawn to screen");

                // This is ugly AF but this works
                for mut line in &self.gfx.clone().into_iter().chunks(8) {
                    let b1 = line.next().unwrap();
                    let b2 = line.next().unwrap();
                    let b3 = line.next().unwrap();
                    let b4 = line.next().unwrap();
                    let b5 = line.next().unwrap();
                    let b6 = line.next().unwrap();
                    let b7 = line.next().unwrap();
                    let b8 = line.next().unwrap();
                    log::log!(Level::Info, "{:0>8b} {:0>8b} {:0>8b} {:0>8b} {:0>8b} {:0>8b} {:0>8b} {:0>8b}", b1, b2,
                            b3, b4, b5, b6, b7, b8);
                }

            },

            // 0xEX9E - Skip if key VX is pressed
            opcode if opcode & 0xF0FF == 0xE09E => {
                if self.keys[(self.registers[x] & 0x0F) as usize] {
                    self.program_counter += 2;
                    log::info!("Skipped to 0x{:0>3X} as the key {x} was pressed", self.program_counter)
                }
            },

            // 0xEXA1 - Skip if key VX is not pressed
            opcode if opcode & 0xF0FF == 0xE0A1 => {
                if !self.keys[(self.registers[x] & 0x0F) as usize] {
                    self.program_counter += 2;
                    log::info!("Skipped to 0x{:0>3X} as the key {x} was not pressed", self.program_counter)
                }
            },

            // 0xFX07 - Set VX to the current value of the delay timer
            opcode if opcode & 0xF0FF == 0xF007 => {
                self.registers[x] = self.delay_timer;
                log::info!("Set register {x} to the value of delay timer {}", self.delay_timer)
            },

            // 0xFX15 - Set the delay timer to VX
            opcode if opcode & 0xF0FF == 0xF015 => {
                self.delay_timer = self.registers[x];
                log::info!("Set the delay timer to the value of register {x} - {}", self.delay_timer)
            },

            // 0xFX18 - Set the sound timer to VX
            opcode if opcode & 0xF0FF == 0xF018 => {
                self.sound_timer = self.registers[x];
                log::info!("Set the sound timer to the value of register {x} - {}", self.delay_timer)
            },

            // 0xFX1E - Add the value in VX to the index register
            opcode if opcode & 0xF0FF == 0xF01E => {
                self.index_register += self.registers[x] as u16;
                if self.index_register > 4095 {
                    self.registers[15] = 0x01;
                    self.index_register -= 4096;
                } else {
                    self.registers[15] = 0x00;
                }
                log::info!("Added the value from register {x} to the index register")
            },

            // 0xFX0A - Wait for a key press and store it in VX
            opcode if opcode & 0xF0FF == 0xF00A => {
                log::info!("Waiting for a key press at 0x{:0>3X}", self.program_counter);
                if self.keys.iter().any(|x| { *x }) {
                    let (index, value) = self.keys.iter()
                        .find_position(|x| { **x }).unwrap();
                    self.registers[x] = index as u8;
                    log::info!("Captured keypress: {index}")
                } else {
                    self.program_counter -= 2;
                }
                
            },

            // 0xFX29 - Set the index register to the position of the hexadecimal character in VX
            opcode if opcode & 0xF0FF == 0xF029 => {
                self.index_register = match self.registers[x] {
                    0x0 => { 0x0050 },
                    0x1 => { 0x0055 },
                    0x2 => { 0x005A },
                    0x3 => { 0x005F },
                    0x4 => { 0x0064 },
                    0x5 => { 0x0069 },
                    0x6 => { 0x006E },
                    0x7 => { 0x0073 },
                    0x8 => { 0x0078 },
                    0x9 => { 0x007D },
                    0xA => { 0x0082 },
                    0xB => { 0x0087 },
                    0xC => { 0x008C },
                    0xD => { 0x0091 },
                    0xE => { 0x0096 },
                    0xF => { 0x009B },
                    _ => { return Err(EmulationErr::InvalidValueInRegister(x as u8, self
                        .registers[x])) }
                };
                log::info!("Set the index register to the position of the character {:X}", self.registers[x])
            },

            // 0xFX33 - Store the Binary-coded decimal value of VX starting at index register
            opcode if opcode & 0xF0FF == 0xF033 => {
                self.memory[self.index_register as usize] = self.registers[x].div(100);
                self.memory[self.index_register as usize + 1] = (self.registers[x] % 100).div(10);
                self.memory[self.index_register as usize + 2] = self.registers[x] % 10;
                log::info!("Stored the BCD value {} starting at position 0x{:0>3X}",
                    self.registers[x], self.index_register)
            },
            
            // 0xFX55 - Store V0 - VX into memory
            opcode if opcode & 0xF0FF == 0xF055 => {
                for offset in 0..=x {
                    self.memory[
                        (self.index_register + offset as u16) as usize
                        ] = self.registers[offset]
                }
                log::info!("Saved values {:?} into memory starting at 0x{:0>3X}",
                    &self.registers[0..=x], self.index_register)
            },
            
            // 0xFX65 - Load into V0 - VX from memory
            opcode if opcode & 0xF0FF == 0xF065 => {
                for offset in 0..=x {
                    self.registers[offset] = self.memory[
                        (self.index_register + offset as u16) as usize
                        ];
                }

                log::info!("Loaded values {:?} from memory starting at 0x{:0>3X}",
                    &self.registers[0..=x], self.index_register)
            },
            
            opcode => {
                if self.quirks.superchip_opcodes {
                    return self.handle_superchip_opcode(opcode, x, y, n, nn, nnn)
                } else {
                    return Err(EmulationErr::UnknownOpcode(self.opcode))
                }
            }
        }
        log::log!(Level::Info, "Executed opcode: 0x{:0>4X}, registers: {:?}, index register: {}",
            self.opcode, self.registers, self.index_register);
        // Update timers

        Ok(())

    }
    fn handle_superchip_opcode(
        &mut self, opcode: u16, x: usize, y: usize, n: u8, nn: u8, nnn: u16
    ) -> Result<(), EmulationErr> {

        match opcode {
            // 0x00CN - Scroll display N lines down
            0x00C0..=0x00CF => {
                self.gfx = [
                    vec![0x00; (8 * n) as usize],
                    self.gfx.clone(),
                ].concat();
            }
            
            // 0x00FB - Scroll display 4 pixels right
            0x00FB => {
                let mut rem: Option<u8>;
                for row in 0..32 {
                    rem = None;
                    for col in 0..8 {
                        let chunk = self.gfx[row * 8 + col];
                        let new_rem = chunk & 0x0F;
                        if rem.is_some() {
                            self.gfx[row * 8 + col] = (chunk >> 4) | (rem.unwrap() << 4);
                        } else {
                            self.gfx[row * 8 + col] = chunk >> 4;
                        }
                        rem = Some(new_rem);
                    }
                }
            }
            
            0x00FD => {
                return Err(EmulationErr::ProgramExited)
            }

            0x00FE => {
                // Disable high-resolution mode
            }

            0x00FF => {
                // Enable high-resolution mode
            }

            // 0xFX75 - Store V0..VX in RPL user flags (X <= 7)
            _ if opcode & 0xF0FF == 0xF075 => {
                if x > 7 {
                    return Err(EmulationErr::InvalidRegisterReference)
                }
                let portion = &self.registers[0..=x];
                self.rpl[0..][..=x].copy_from_slice(portion);
            }
            
            // 0xFX85 - Read V0..VX from RPL user flags (X <= 7)
            _ if opcode & 0xF0FF == 0xF075 => {
                if x > 7 {
                    return Err(EmulationErr::InvalidRegisterReference)
                }
                let portion = &self.rpl[0..=x];
                self.registers[0..][..=x].copy_from_slice(portion);
            }

            _ => {
                return Err(EmulationErr::UnknownOpcode(opcode))
            }
        }

        Ok(())
    }
}