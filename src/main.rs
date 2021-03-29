use pixels::{Error, Pixels, SurfaceTexture};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::env;
use std::fs;
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const BACKGROUND: [u8; 4] = [0x0e, 0x0e, 0x0e, 0xff];
const FOREGROUND: [u8; 4] = [0x00, 0xf0, 0x00, 0xff];
const FONT_SPRITES: [u8; 80] = [
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
const ASM: bool = false;

#[derive(Debug)]
struct Machine<const W: usize, const H: usize> {
    memory: [u8; 4096], //guess what
    v: [u8; 16],        //general purpose registers
    i: usize,           //memory indexing register
    dt: u8,             //delay timer
    st: u8,             //sound timer
    pc: usize,          //program counter
    sp: usize,          //stack pointer
    stack: [usize; 16],
    display: [[bool; W]; H],
    keyboard: [bool; 16],
    last_update: Instant,
    timing_error: Duration,
    dirty: bool,
    rnd: ThreadRng,
}

impl<const W: usize, const H: usize> Machine<W, H> {
    fn new() -> Self {
        let mut this = Self {
            memory: [0; 4096], // guess what
            v: [0; 16],        // general purpose registers
            i: 0,
            dt: 0, // delay timer
            st: 0, // sound timer
            //snd: [0; 2], // this or dt & st
            pc: 0x200, //program counter
            sp: 0,     //stack pointer
            stack: [0; 16],
            display: [[false; W]; H],
            keyboard: [false; 16],
            last_update: Instant::now(),
            timing_error: Duration::from_secs(0),
            dirty: true,
            rnd: thread_rng(),
        };
        this.memory[0..FONT_SPRITES.len()].copy_from_slice(&FONT_SPRITES);
        this
    }
    fn update(&mut self) {
        //std::thread::sleep(Duration::from_secs_f64(0.1));
        let timer_delay: Duration = Duration::from_secs_f64(1. / 60.);
        let opcode: u16 = (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16;
        //println!("{:04x}", opcode);
        //let opcode = 0x1000_u16;
        self.pc += 2;
        let x = (opcode >> 8 & 0xF) as usize;
        let y = (opcode >> 4 & 0xF) as usize;
        let z = (opcode & 0xF) as usize;
        let k = (opcode & 0xFF) as u8;
        let n = opcode & 0xFFF;
        if self.timing_error + self.last_update.elapsed() > timer_delay {
            let mut times = 0;
            while self.timing_error > timer_delay {
                self.timing_error -= timer_delay;
                times += 1;
            }

            if self.dt > 0 {
                self.dt -= times.min(self.dt);
            }
            if self.st > 0 {
                self.st -= times.min(self.st);
            }
            self.last_update = Instant::now();
        }
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }

        //println!("{:03x}: {:04x}", self.pc, opcode);
        match (opcode >> 12 & 0xF) as u8 {
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
            0 => match k {
                0xE0 => {
                    //CLS
                    if ASM {
                        println!("{:04x}(CLS)", opcode);
                    }
                    self.display = [[false; W]; H];
                }
                0xEE => {
                    self.sp -= 1;
                    if ASM {
                        println!(
                            "{:04x}(RET) sp:{:02x} s[sp]:{:03x}",
                            opcode, self.sp, self.stack[self.sp]
                        );
                    }
                    self.pc = self.stack[self.sp];
                }
                _ => {}
            },

            //# 1nnn - JP addr
            //Jump to location nnn.
            //
            //The interpreter sets the program counter to nnn.
            0x1 => {
                if ASM {
                    println!("{:04x}(JP) {:03x}", opcode, n);
                }
                self.pc = n as usize;
            }

            //# 2nnn - CALL addr
            //Call subroutine at nnn.
            //
            //The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
            0x2 => {
                if ASM {
                    println!("{:04x}(CALL) {:03x}", opcode, n);
                }
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = n as usize;
            }

            //# 3xkk - SE Vx, byte
            //Skip next instruction if Vx = kk.
            //
            //The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
            0x3 => {
                if ASM {
                    println!("{:04x}(SE) {:02x}=={:02x}", opcode, self.v[x], k);
                }
                if self.v[x] == k {
                    self.pc += 2;
                }
            }

            //# 4xkk - SNE Vx, byte
            //Skip next instruction if Vx != kk.
            //
            //The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
            0x4 => {
                if ASM {
                    println!("{:04x}(SNE) {:02x}!={:02x}", opcode, self.v[x], k);
                }
                if self.v[x] != k {
                    self.pc += 2;
                }
            }

            //# 5xy0 - SE Vx, Vy
            //Skip next instruction if Vx = Vy.
            //
            //The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
            0x5 => {
                if ASM {
                    println!("{:04x}(SE) {:02x}=={:02x}", opcode, self.v[x], self.v[y]);
                }
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }

            //# 6xkk - LD Vx, byte
            //Set Vx = kk.
            //
            //The interpreter puts the value kk into register Vx.
            0x6 => {
                if ASM {
                    println!("{:04x}(LD) v[{:01x}]={:02x}", opcode, x, k);
                }
                self.v[x] = k;
            }

            //# 7xkk - ADD Vx, byte
            //Set Vx = Vx + kk.
            //
            //Adds the value kk to the value of register Vx, then stores the result in Vx.
            0x7 => {
                if ASM {
                    println!("{:04x}(ADD) v[{:01x}]={:02x}", opcode, x, k);
                }
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
                        self.v[0xf] = if carry { 1 } else { 0 };
                        sum
                    }

                    //# 8xy5 - SUB Vx, Vy
                    //Set Vx = Vx - Vy, set VF = NOT borrow.
                    //
                    //If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
                    0x5 => {
                        let (diff, borrow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[0xf] = if !borrow { 1 } else { 0 };
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
                        self.v[0xf] = if !borrow { 1 } else { 0 };
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
                    _ => panic!("Invalid opcode {}", opcode),
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
            0xA => self.i = n as usize,

            //# Bnnn - JP V0, addr
            //Jump to location nnn + V0.
            //
            //The program counter is set to nnn plus the value of V0.
            0xB => {
                self.pc = self.v[0] as usize + n as usize;
            }

            //# Cxkk - RND Vx, byte
            //Set Vx = random byte AND kk.
            //
            //The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk. The results are stored in Vx. See instruction 8xy2 for more information on AND.
            0xC => {
                self.v[x] = self.rnd.gen::<u8>() & k;
                println!("{}", self.v[x]);
            }

            //# Dxyn - DRW Vx, Vy, nibble
            //Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
            //
            //The interpreter reads n bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen at coordinates (Vx, Vy). Sprites are XORed onto the existing screen. If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen. See instruction 8xy3 for more information on XOR, and section 2.4, Display, for more information on the Chip-8 screen and sprites.
            0xD => {
                self.v[0xf] = 0;
                for dy in 0..z {
                    for dx in 0..8 {
                        if self.memory[self.i + dy] & 0x80 >> dx > 0 {
                            self.dirty = true;
                            let nx = (self.v[x] as usize + dx) % W;
                            let ny = (self.v[y] as usize + dy) % H;
                            self.display[ny][nx] = if self.display[ny][nx] {
                                self.v[0xf] = 1;
                                false
                            } else {
                                true
                            }
                        }
                    }
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
            0xE => match k {
                0x9e => {
                    if self.keyboard[self.v[x] as usize] {
                        self.pc += 2;
                    }
                }
                0xa1 => {
                    if !self.keyboard[self.v[x] as usize] {
                        self.pc += 2;
                    }
                }
                _ => {}
            },

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
                    0x0a => {
                        let mut pressed = false;
                        for (i, key) in self.keyboard.iter().enumerate() {
                            if *key {
                                self.v[x] = i as u8;
                                pressed = true;
                                break;
                            }
                        }
                        if !pressed {
                            self.pc -= 2;
                        }
                    }

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
                        self.memory[self.i + 1] = (x / 10) % 10;
                        self.memory[self.i + 2] = x % 10;
                    }

                    //# Fx55 - LD [I], Vx
                    //Store registers V0 through Vx in memory starting at location I.
                    //
                    //The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
                    0x55 => {
                        //self.memory[self.i + 0..=self.i + x].copy_from_slice(&self.v[0..=x]);
                        for i in 0..=x {
                            self.memory[self.i + i] = self.v[i];
                        }
                    }

                    //# Fx65 - LD Vx, [I]
                    //Read registers V0 through Vx from memory starting at location I.
                    //
                    //The interpreter reads values from memory starting at location I into registers V0 through Vx.
                    0x65 => {
                        //self.v[0..=x].copy_from_slice(&self.memory[self.i + 0..=self.i + x]);
                        for i in 0..=x {
                            self.v[i] = self.memory[self.i + i];
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, frame: &mut [u8]) {
        if self.dirty {
            self.dirty = false;
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let x = i % W;
                let y = i / W;

                let rgba = if self.display[y][x] {
                    FOREGROUND
                } else {
                    BACKGROUND
                };

                pixel.copy_from_slice(&rgba);
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let file: &str = &env::args().collect::<Vec<_>>()[1];
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Chip-8")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    let mut m = Machine::<WIDTH, HEIGHT>::new();
    let f = fs::read(file).unwrap();
    m.memory[0x200..0x200 + f.len()].copy_from_slice(&f);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        if let Event::RedrawRequested(_) = event {
            let frame = pixels.get_frame();
            m.draw(frame);
            if pixels
                .render()
                .map_err(|e| {
                    dbg!("pixels.render() failed:");
                    dbg!(e);
                })
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            let keymap = [
                VirtualKeyCode::Z,    //0
                VirtualKeyCode::Key4, //1
                VirtualKeyCode::Key5, //2
                VirtualKeyCode::Key6, //3
                VirtualKeyCode::C,    //4
                VirtualKeyCode::W,    //5
                VirtualKeyCode::K,    //6
                VirtualKeyCode::E,    //7
                VirtualKeyCode::O,    //8
                VirtualKeyCode::S,    //9
                VirtualKeyCode::P,    //10
                VirtualKeyCode::B,    //11
                VirtualKeyCode::Key7, //12
                VirtualKeyCode::H,    //13
                VirtualKeyCode::N,    //14
                VirtualKeyCode::M,    //15
            ];
            for (i, key) in keymap.iter().enumerate() {
                m.keyboard[i] = input.key_held(*key);
            }

            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }
        }
        m.update();
        window.request_redraw();
    });
}
