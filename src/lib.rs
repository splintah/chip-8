//! CHIP-8
//!
//! See Cowgod's [CHIP-8 technical reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM) for
//! a specification for the CHIP-8 processor.

extern crate rand;

use self::rand::rngs::SmallRng;
use self::rand::{FromEntropy, Rng};

/// The width of a CHIP-8 display.
pub const WIDTH: usize = 64;
/// The height of a CHIP-8 display.
pub const HEIGHT: usize = 32;
/// The CHIP-8 font for characters 0-9 and A-F.
pub const FONTSET: [u8; 80] = [
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
];

/// The `Error` type returned when an error occurred in `Processor::run_cycle`.
pub enum Error {
    /// A `String` error.
    Error(String),
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::Error(s)
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Error::Error(e) => write!(f, "{}", e),
        }
    }
}

impl ::std::fmt::Debug for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Error::Error(e) => write!(f, "{}", e),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Error(e) => &e,
        }
    }
}

/// The CHIP-8 processor.
#[derive(Clone)]
pub struct Processor {
    /// The processor's memory.
    pub memory: [u8; 4096],
    /// The registers.
    pub registers: [u8; 16],
    /// The index, which points at an element of memory.
    pub index: usize,
    /// The index in the memory which points to the current opcode.
    pub program_counter: usize,
    /// The display.
    pub display: [bool; WIDTH * HEIGHT],
    /// Whether to update the display.
    pub draw: bool,
    /// The delay timer.
    pub delay_timer: u8,
    /// The sound timer.
    pub sound_timer: u8,
    /// The stack.
    pub stack: [u16; 16],
    /// The index which points at the top of the stack.
    pub stack_pointer: usize,
    /// Keypad with 16 keys which can be pressed (`true`) or not (`false`).
    ///
    /// # Example mapping
    /// ```plain
    /// Keypad         Keyboard
    /// +-+-+-+-+      +-+-+-+-+
    /// |1|2|3|C|      |1|2|3|4|
    /// +-+-+-+-+      +-+-+-+-+
    /// |4|5|6|D|      |Q|W|E|R|
    /// +-+-+-+-+  =>  +-+-+-+-+
    /// |7|8|9|E|      |A|S|D|F|
    /// +-+-+-+-+      +-+-+-+-+
    /// |A|0|B|F|      |Z|X|C|V|
    /// +-+-+-+-+      +-+-+-+-+
    /// ```
    pub keypad: [bool; 16],
    /// The random number generator (RNG).
    rng: SmallRng,
}

impl Processor {
    /// Create a new `Processor`.
    pub fn new() -> Processor {
        Processor::default()
    }

    /// Create a new `Processor` and load `file` into memory.
    pub fn with_file(file: &[u8]) -> Processor {
        let mut processor = Processor::default();
        processor.load_file(file);
        processor
    }

    /// Load `file` into memory.
    pub fn load_file(&mut self, file: &[u8]) {
        self.memory[0x200..0x200 + file.len()].copy_from_slice(&file);
    }

    /// Set the state of a key.
    pub fn set_key(&mut self, key: usize, pressed: bool) {
        self.keypad[key] = pressed;
    }

    /// Get the current `opcode`.
    pub fn opcode(&self) -> u16 {
        (self.memory[self.program_counter] as u16) << 8
            | self.memory[self.program_counter + 1] as u16
    }

    /// Emulate a processor cycle.
    pub fn run_cycle(&mut self) -> Result<(), Error> {
        // V![$index] is the register at $index.
        macro_rules! V {
            [ $index:expr ] => { self.registers[$index] };
        }

        let opcode = self.opcode();

        self.program_counter += 2;

        let x: usize = (opcode as usize & 0x0F00) >> 8;
        let y: usize = (opcode as usize & 0x00F0) >> 4;
        let n: u8 = opcode as u8 & 0x000F;
        let kk: u8 = opcode as u8;
        let nnn: usize = opcode as usize & 0x0FFF;

        match (opcode & 0xF000) >> 12 {
            0x0 => match opcode & 0x00FF {
                // 00E0 - CLS
                // Clear the display.
                0xE0 => {
                    self.display = [false; WIDTH * HEIGHT];
                    self.draw = true;
                }
                // 00EE - RET
                // Return from a subroutine.
                // The interpreter sets the program counter to the address at the top of the stack,
                // then subtracts 1 from the stack pointer.
                0xEE => {
                    self.stack_pointer -= 1;
                    self.program_counter = self.stack[self.stack_pointer] as usize;
                }
                // 0nnn - SYS addr
                // Jump to a machine code routine at nnn.
                // This instruction is only used on the old computers on which Chip-8 was originally
                // implemented. It is ignored by modern interpreters.
                _ => {}
            },
            // 1nnn - JP addr
            // Jump to location nnn.
            // The interpreter sets the program counter to nnn.
            0x1 => self.program_counter = nnn,
            // 2nnn - CALL addr
            // Call subroutine at nnn.
            // The interpreter increments the stack pointer, then puts the current PC on the top of
            // the stack. The PC is then set to nnn.
            0x2 => {
                self.stack[self.stack_pointer] = self.program_counter as u16;
                self.stack_pointer += 1;
                self.program_counter = nnn;
            }
            // 3xkk - SE Vx, byte
            // Skip next instruction if Vx = kk.
            // The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
            0x3 => if V![x] == kk {
                self.program_counter += 2
            },
            // 4xkk - SNE Vx, byte
            // Skip next instruction if Vx != kk.
            // The interpreter compares register Vx to kk, and if they are not equal, increments the
            // program counter by 2.
            0x4 => if V![x] != kk {
                self.program_counter += 2;
            },
            // 5xy0 - SE Vx, Vy
            // Skip next instruction if Vx = Vy.
            // The interpreter compares register Vx to register Vy, and if they are equal,
            // increments the program counter by 2.
            0x5 => if V![x] == V![y] {
                self.program_counter += 2;
            },
            // 6xkk - LD Vx, byte
            // Set Vx = kk.
            // The interpreter puts the value kk into register Vx.
            0x6 => V![x] = kk,
            // 7xkk - ADD Vx, byte
            // Set Vx = Vx + kk.
            // Adds the value kk to the value of register Vx, then stores the result in Vx.
            0x7 => V![x] = V![x].wrapping_add(kk),
            0x8 => match opcode & 0x000F {
                // 8xy0 - LD Vx, Vy
                // Set Vx = Vy.
                // Stores the value of register Vy in register Vx.
                0x0 => V![x] = V![y],
                // 8xy1 - OR Vx, Vy
                // Set Vx = Vx OR Vy.
                // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
                // A bitwise OR compares the corresponding bits from two values, and if either bit
                // is 1, then the same bit in the result is also 1. Otherwise, it is 0.
                0x1 => V![x] |= V![y],
                // 8xy2 - AND Vx, Vy
                // Set Vx = Vx AND Vy.
                // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
                // A bitwise AND compares the corresponding bits from two values, and if both bits
                // are 1, then the same bit in the result is also 1. Otherwise, it is 0.
                0x2 => V![x] &= V![y],
                // 8xy3 - XOR Vx, Vy
                // Set Vx = Vx XOR Vy.
                // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the
                // result in Vx. An exclusive OR compares the corresponding bits from two values,
                // and if the bits are not both the same, then the corresponding bit in the result
                // is set to 1. Otherwise, it is 0.
                0x3 => V![x] ^= V![y],
                // 8xy4 - ADD Vx, Vy
                // Set Vx = Vx + Vy, set VF = carry.
                // The values of Vx and Vy are added together. If the result is greater than 8 bits
                // (i.e., > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result
                // are kept, and stored in Vx.
                0x4 => {
                    let (value, carry) = V![x].overflowing_add(V![y]);
                    V![0xF] = if carry { 1 } else { 0 };
                    V![x] = value;
                }
                // 8xy5 - SUB Vx, Vy
                // Set Vx = Vx - Vy, set VF = NOT borrow.
                // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and
                // the results stored in Vx.
                0x5 => {
                    let (value, borrow) = V![x].overflowing_sub(V![y]);
                    V![0xF] = if borrow { 0 } else { 1 };
                    V![x] = value;
                }
                // 8xy6 - SHR Vx {, Vy}
                // Set Vx = Vx SHR 1.
                // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then
                // Vx is divided by 2.
                0x6 => {
                    V![0xF] = V![x] & 0x1;
                    V![x] >>= 1;
                }
                // 8xy7 - SUBN Vx, Vy
                // Set Vx = Vy - Vx, set VF = NOT borrow.
                // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and
                // the results stored in Vx.
                0x7 => {
                    let (value, borrow) = V![y].overflowing_sub(V![x]);
                    V![0xF] = if borrow { 0 } else { 1 };
                    V![x] = value;
                }
                // 8xyE - SHL Vx {, Vy}
                // Set Vx = Vx SHL 1.
                // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then
                // Vx is multiplied by 2.
                0xE => {
                    V![0xF] = if V![x] & 0x80 == 1 << 7 { 1 } else { 0 };
                    V![x] <<= 1;
                }
                _ => {
                    return Err(format!(
                        "Unknown opcode at 0x{:X}: 0x{:04X}.",
                        self.program_counter, opcode
                    ).into())
                }
            },
            // 9xy0 - SNE Vx, Vy
            // Skip next instruction if Vx != Vy.
            // The values of Vx and Vy are compared, and if they are not equal, the program counter
            // is increased by 2.
            0x9 => if V![x] != V![y] {
                self.program_counter += 2;
            },
            // Annn - LD I, addr
            // Set I = nnn.
            // The value of register I is set to nnn.
            0xA => self.index = nnn,
            // Bnnn - JP V0, addr
            // Jump to location nnn + V0.
            // The program counter is set to nnn plus the value of V0.
            0xB => self.program_counter = V![0] as usize + nnn,
            // Cxkk - RND Vx, byte
            // Set Vx = random byte AND kk.
            // The interpreter generates a random number from 0 to 255, which is then ANDed with the
            // value kk. The results are stored in Vx. See instruction 8xy2 for more information on
            // AND.
            0xC => V![x] = self.rng.gen::<u8>() & kk,
            // Dxyn - DRW Vx, Vy, nibble
            // Display n-byte sprite starting at memory location I at (Vx, Vy), set
            // VF = collision.
            // The interpreter reads n bytes from memory, starting at the address stored in I. These
            // bytes are then displayed as sprites on screen at coordinates (Vx, Vy). Sprites are
            // XORed onto the existing screen. If this causes any pixels to be erased, VF is set to
            // 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the
            // coordinates of the display, it wraps around to the opposite side of the screen. See
            // instruction 8xy3 for more information on XOR, and section 2.4, Display, for more
            // information on the Chip-8 screen and sprites.
            0xD => {
                self.draw = true;
                V![0xF] = 0;
                for col in 0..n as usize {
                    let pixel = self.memory[self.index + col];
                    for row in 0..8 {
                        if pixel & (0x80 >> row) != 0 {
                            let x_coord = (V![x] as usize + row) % WIDTH;
                            let y_coord = (V![y] as usize + col) % HEIGHT;
                            let index = x_coord + y_coord * WIDTH;

                            if self.display[index] {
                                V![0xF] = 1;
                            }
                            self.display[index] ^= true;
                        }
                    }
                }
            }
            0xE => match opcode & 0x00FF {
                // Ex9E - SKP Vx
                // Skip next instruction if key with the value of Vx is pressed.
                // Checks the keyboard, and if the key corresponding to the value of Vx is currently
                // in the down position, PC is increased by 2.
                0x9E => if self.keypad[V![x] as usize] {
                    self.program_counter += 2;
                },
                // ExA1 - SKNP Vx
                // Skip next instruction if key with the value of Vx is not pressed.
                // Checks the keyboard, and if the key corresponding to the value of Vx is currently
                // in the up position, PC is increased by 2.
                0xA1 => if !self.keypad[V![x] as usize] {
                    self.program_counter += 2;
                },
                _ => {
                    return Err(format!(
                        "Unknown opcode at 0x{:X}: 0x{:04X}.",
                        self.program_counter, opcode
                    ).into())
                }
            },
            0xF => match opcode & 0x00FF {
                // Fx07 - LD Vx, DT
                // Set Vx = delay timer value.
                // The value of DT is placed into Vx.
                0x07 => V![x] = self.delay_timer,
                // Fx0A - LD Vx, K
                // Wait for a key press, store the value of the key in Vx
                // All execution stops until a key is pressed, then the value of that key is stored
                // in Vx.
                0x0A => {
                    let mut key_press = false;
                    for (i, key) in self.keypad.iter().enumerate() {
                        if *key {
                            V![x] = i as u8;
                            key_press = true;
                            break;
                        }
                    }

                    if !key_press {
                        self.program_counter -= 2;
                    }
                }
                // Fx15 - LD DT, Vx
                // Set delay timer = Vx.
                // DT is set equal to the value of Vx.
                0x15 => self.delay_timer = V![x],
                // Fx18 - LD ST, Vx
                // Set sound timer = Vx.
                // ST is set equal to the value of Vx.
                0x18 => self.sound_timer = V![x],
                // Fx1E - ADD I, Vx
                // Set I = I + Vx.
                // The values of I and Vx are added, and the results are stored in I.
                0x1E => self.index += V![x] as usize,
                // Fx29 - LD F, Vx
                // Set I = location of sprite for digit Vx.
                // The value of I is set to the location for the hexadecimal sprite corresponding to
                // the value of Vx. See section 2.4, Display, for more information on the Chip-8
                // hexadecimal font.
                0x29 => self.index = 5 * V![x] as usize,
                // Fx33 - LD B, Vx
                // Store BCD representation of Vx in memory locations I, I+1, and I+2.
                // The interpreter takes the decimal value of Vx, and places the hundreds digit in
                // memory at location in I, the tens digit at location I+1, and the ones digit at
                // location I+2.
                0x33 => {
                    self.memory[self.index] = V![x] / 100;
                    self.memory[self.index + 1] = (V![x] / 10) % 10;
                    self.memory[self.index + 2] = V![x] % 10;
                }
                // Fx55 - LD [I], Vx
                // Store registers V0 through Vx in memory starting at location I. The interpreter
                // copies the values of registers V0 through Vx into memory, starting at the address
                // in I.
                0x55 => self.memory[self.index..self.index + x + 1]
                    .copy_from_slice(&self.registers[0x0..x + 1]),
                // Fx65 - LD Vx, [I]
                // Read registers V0 through Vx from memory starting at location I. The interpreter
                // reads values from memory starting at location I into registers V0 through Vx.
                0x65 => self.registers[0x0..x + 1]
                    .copy_from_slice(&self.memory[self.index..self.index + x + 1]),
                _ => {
                    return Err(format!(
                        "Unknown opcode at 0x{:X}: 0x{:04X}.",
                        self.program_counter, opcode
                    ).into())
                }
            },
            _ => {
                return Err(format!(
                    "Unknown opcode at 0x{:X}: 0x{:04X}.",
                    self.program_counter, opcode
                ).into())
            }
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        Ok(())
    }
}

impl Default for Processor {
    fn default() -> Processor {
        let mut memory = [0; 4096];
        memory[..80].copy_from_slice(&FONTSET);
        Processor {
            memory,
            registers: [0; 16],
            index: 0,
            program_counter: 0x200,
            display: [false; WIDTH * HEIGHT],
            draw: true,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keypad: [false; 16],
            rng: SmallRng::from_entropy(),
        }
    }
}
