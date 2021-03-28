//use pixels;
use rand::Rng;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct Machine<const W: usize, const H: usize> {
    memory: [u8; 4096], // guess what
    v: [u8; 16],        // general purpose registers
    i: usize,
    dt: u8, // delay timer
    st: u8, // sound timer
    //snd: [u8; 2], // this or dt & st
    pc: usize, //program counter
    sp: usize, //stack pointer
    stack: [usize; 16],
    // display is hard coded to 64x32 might add 64x48, 64x64 and or 128x64
    display: [[bool; W]; H],
    keyboard: [bool; 16],
}

impl<const W: usize, const H: usize> Machine<W, H> {
    fn step(&mut self) {
        let instruction: u16 = self.memory[self.pc] as u16 | (self.memory[self.pc + 1] as u16) << 8;
        //let instruction = 0x1000_u16;
        self.pc += 2;
        let x = (instruction >> 8 & 0xF) as usize;
        let y = (instruction >> 4 & 0xF) as usize;
        let z = (instruction & 0xF) as usize;
        let k = (instruction & 0xFF) as u8;
        let n = instruction & 0xFFF;

        match (instruction >> 12 & 0xF) as u8 {
            //# 0nnn - SYS addr
            //Jump to a machine code routine at nnn.
            //
            //This instruction is only used on the old computers on which Chip-8 was originally implemented. It is ignored by modern interpreters.
            //
            //# 00E0 - CLS
            //Clear the display.
            //
            //# 00EE - RET
            //Return from a subroutine.
            //
            //The interpreter sets the program counter to the address at the top of the stack, then subtracts 1 from the stack pointer.
            0 => {
                if instruction == 0x00E0 {
                    //CLS
                    self.display = [[false; W]; H];
                }
                if instruction == 0x00EE {
                    self.sp -= 1;
                    self.pc = self.stack[self.sp];
                }
            }

            //# 1nnn - JP addr
            //Jump to location nnn.
            //
            //The interpreter sets the program counter to nnn.
            0x1 => {
                self.pc = n as usize;
            }

            //# 2nnn - CALL addr
            //Call subroutine at nnn.
            //
            //The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
            0x2 => {
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = n as usize;
            }

            //# 3xkk - SE Vx, byte
            //Skip next instruction if Vx = kk.
            //
            //The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
            0x3 => {
                if self.v[x] == k {
                    self.pc += 2;
                }
            }

            //# 4xkk - SNE Vx, byte
            //Skip next instruction if Vx != kk.
            //
            //The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
            0x4 => {
                if self.v[x] != k {
                    self.pc += 2;
                }
            }

            //# 5xy0 - SE Vx, Vy
            //Skip next instruction if Vx = Vy.
            //
            //The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
            0x5 => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }

            //# 6xkk - LD Vx, byte
            //Set Vx = kk.
            //
            //The interpreter puts the value kk into register Vx.
            0x6 => {
                self.v[x] = k;
            }

            //# 7xkk - ADD Vx, byte
            //Set Vx = Vx + kk.
            //
            //Adds the value kk to the value of register Vx, then stores the result in Vx.
            0x7 => {
                self.v[x] = self.v[x].wrapping_add(k);
            }

            0x8 => {
                self.v[x] = match z {
                    //# 8xy0 - LD Vx, Vy
                    //Set Vx = Vy.
                    //
                    //Stores the value of register Vy in register Vx.
                    0x0 => self.v[y],

                    //# 8xy1 - OR Vx, Vy
                    //Set Vx = Vx OR Vy.
                    //
                    //Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx. A bitwise OR compares the corrseponding bits from two values, and if either bit is 1, then the same bit in the result is also 1. Otherwise, it is 0.
                    0x1 => self.v[x] | self.v[y],

                    //# 8xy2 - AND Vx, Vy
                    //Set Vx = Vx AND Vy.
                    //
                    //Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx. A bitwise AND compares the corrseponding bits from two values, and if both bits are 1, then the same bit in the result is also 1. Otherwise, it is 0.
                    0x2 => self.v[x] & self.v[y],

                    //# 8xy3 - XOR Vx, Vy
                    //Set Vx = Vx XOR Vy.
                    //
                    //Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx. An exclusive OR compares the corrseponding bits from two values, and if the bits are not both the same, then the corresponding bit in the result is set to 1. Otherwise, it is 0.
                    0x3 => self.v[x] ^ self.v[y],

                    //# 8xy4 - ADD Vx, Vy
                    //Set Vx = Vx + Vy, set VF = carry.
                    //
                    //The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result are kept, and stored in Vx.
                    0x4 => {
                        let (sum, carry) = self.v[x].overflowing_add(self.v[y]);
                        self.v[0xf] = carry as u8;
                        sum
                    }

                    //# 8xy5 - SUB Vx, Vy
                    //Set Vx = Vx - Vy, set VF = NOT borrow.
                    //
                    //If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
                    0x5 => {
                        let (diff, borrow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[0xf] = borrow as u8;
                        diff
                    }

                    //# 8xy6 - SHR Vx {, Vy}
                    //Set Vx = Vx SHR 1.
                    //
                    //If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
                    0x6 => {
                        self.v[0xf] = self.v[x] & 1;
                        self.v[x] >> 1
                    }

                    //# 8xy7 - SUBN Vx, Vy
                    //Set Vx = Vy - Vx, set VF = NOT borrow.
                    //
                    //If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
                    0x7 => {
                        let (diff, borrow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[0xf] = borrow as u8;
                        diff
                    }

                    //# 8xyE - SHL Vx {, Vy}
                    //Set Vx = Vx SHL 1.
                    //
                    //If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
                    0xE => {
                        self.v[0xf] = (self.v[x] >> 7) & 1;
                        self.v[x] << 1
                    }
                    _ => panic!("Invalid opcode {}", instruction),
                }
            }

            //# 9xy0 - SNE Vx, Vy
            //Skip next instruction if Vx != Vy.
            //
            //The values of Vx and Vy are compared, and if they are not equal, the program counter is increased by 2.
            0x9 => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }

            //# Annn - LD I, addr
            //Set I = nnn.
            //
            //The value of register I is set to nnn.
            0xA => {
                self.i = n as usize;
            }

            //# Bnnn - JP V0, addr
            //Jump to location nnn + V0.
            //
            //The program counter is set to nnn plus the value of V0.
            0xB => {
                self.pc = (self.v[0] as u16 + n) as usize;
            }

            //# Cxkk - RND Vx, byte
            //Set Vx = random byte AND kk.
            //
            //The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk. The results are stored in Vx. See instruction 8xy2 for more information on AND.
            0xC => {
                self.v[x] = rand::thread_rng().gen::<u8>() & k;
            }

            //# Dxyn - DRW Vx, Vy, nibble
            //Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
            //
            //The interpreter reads n bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen at coordinates (Vx, Vy). Sprites are XORed onto the existing screen. If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen. See instruction 8xy3 for more information on XOR, and section 2.4, Display, for more information on the Chip-8 screen and sprites.
            0xD => {
                for _i in 0..z {
                    //TOD
                }
            }

            //# Ex9E - SKP Vx
            //Skip next instruction if key with the value of Vx is pressed.
            //
            //Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.
            //
            //# ExA1 - SKNP Vx
            //Skip next instruction if key with the value of Vx is not pressed.
            //
            //Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.
            0xE => {
                if self.keyboard[self.v[x] as usize] {
                    self.pc += 2;
                }
            }

            0xF => {
                match k {
                    //# Fx07 - LD Vx, DT
                    //Set Vx = delay timer value.
                    //
                    //The value of DT is placed into Vx.
                    0x07 => {
                        self.v[x] = self.dt;
                    }

                    //# Fx0A - LD Vx, K
                    //Wait for a key press, store the value of the key in Vx.
                    //
                    //All execution stops until a key is pressed, then the value of that key is stored in Vx.
                    0x0a => loop {
                        match event::read().unwrap() {
                            Event::Key(event) => {
                                let key = match event.code {
                                    KeyCode::Char('1') => 0,
                                    KeyCode::Char('2') => 1,
                                    KeyCode::Char('3') => 2,
                                    KeyCode::Char('4') => 3,
                                    KeyCode::Char('x') => 4,
                                    KeyCode::Char('v') => 5,
                                    KeyCode::Char('l') => 6,
                                    KeyCode::Char('c') => 7,
                                    KeyCode::Char('u') => 8,
                                    KeyCode::Char('i') => 9,
                                    KeyCode::Char('a') => 10,
                                    KeyCode::Char('e') => 11,
                                    KeyCode::Char('ü') => 12,
                                    KeyCode::Char('ö') => 13,
                                    KeyCode::Char('ä') => 14,
                                    KeyCode::Char('p') => 15,
                                    _ => panic!("unexpected key"),
                                };
                            }
                            _ => {
                                panic!("unexpected evenet")
                            }
                        }
                    },

                    //# Fx15 - LD DT, Vx
                    //Set delay timer = Vx.
                    //
                    //DT is set equal to the value of Vx.
                    0x15 => {
                        self.dt = self.v[x];
                    }

                    //# Fx18 - LD ST, Vx
                    //Set sound timer = Vx.
                    //
                    //ST is set equal to the value of Vx.
                    0x18 => {
                        self.st = self.v[x];
                    }

                    //# Fx1E - ADD I, Vx
                    //Set I = I + Vx.
                    //
                    //The values of I and Vx are added, and the results are stored in I.
                    0x1e => {
                        self.i += self.v[x] as usize;
                    }

                    //# Fx29 - LD F, Vx
                    //Set I = location of sprite for digit Vx.
                    //
                    //The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx. See section 2.4, Display, for more information on the Chip-8 hexadecimal font.
                    0x29 => {
                        self.i = (self.v[x] * 5) as usize;
                    }

                    //# Fx33 - LD B, Vx
                    //Store BCD representation of Vx in memory locations I, I+1, and I+2.
                    //
                    //The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                    0x33 => {
                        let x = self.v[x];
                        self.memory[self.i] = x / 100;
                        self.memory[self.i] = (x / 10) % 10;
                        self.memory[self.i] = x % 10;
                    }

                    //# Fx55 - LD [I], Vx
                    //Store registers V0 through Vx in memory starting at location I.
                    //
                    //The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
                    0x55 => {
                        self.memory[self.i + 0..=self.i + x].copy_from_slice(&self.v[0..=x]);
                        //for i in 0..=x {
                        //self.memory[self.i + i] = self.v[i];
                        //}
                    }

                    //# Fx65 - LD Vx, [I]
                    //Read registers V0 through Vx from memory starting at location I.
                    //
                    //The interpreter reads values from memory starting at location I into registers V0 through Vx.
                    0x65 => {
                        self.v[0..=x].copy_from_slice(&self.memory[self.i + 0..=self.i + x]);
                        //for i in 0..=x {
                        //self.v[i] = self.memory[self.i + i];
                        //}
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn print(&self) -> Result<(), crossterm::ErrorKind> {
        for (i, line) in self.display.iter().enumerate() {
            queue!(stdout(), MoveTo(0, i as u16))?;
            for pixel in line {
                if *pixel {
                    queue!(stdout(), Print("#"))?;
                } else {
                    queue!(stdout(), Print(" "))?;
                }
            }
        }
        stdout().flush()?;
        Ok(())
    }
}

fn main() {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;
    let (cols, rows) = size().expect("no size no game");
    if (cols as usize) < WIDTH || (rows as usize) < HEIGHT {
        panic!("Need a bigger screen");
    };
    queue!(stdout(), terminal::EnterAlternateScreen).unwrap();
    let mut m = Machine {
        memory: [0; 4096], // guess what
        v: [0; 16],        // general purpose registers
        i: 0,
        dt: 0, // delay timer
        st: 0, // sound timer
        //snd: [0; 2], // this or dt & st
        pc: 0, //program counter
        sp: 0, //stack pointer
        stack: [0; 16],
        // display is hard coded to 64x32 might add 64x48, 64x64 and or 128x64
        display: [[false; 64]; 32],
        keyboard: [false; 16],
    };

    loop {
        m.step();
        m.print().unwrap();
        sleep(Duration::from_millis(16));
    }
}
