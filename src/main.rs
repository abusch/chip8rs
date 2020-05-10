use std::path::Path;

use log::{debug, info};
use minifb::{Key, Window, WindowOptions, Scale, ScaleMode};

struct Chip8 {
    pc: u16,
    ram: Ram,
    regs: Registers,
    gfx: Gfx,
    stack: Stack,
    delay_timer: u8,
    sound_timer: u8,
    keys: [bool; 16],
}

impl Chip8 {
    const FONT_DATA_ADDR: u16 = 0x0050;
    const PROG_ADDR: u16 = 0x0200;

    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let rom = std::fs::read(path)?;
        let mut ram = Ram::default();
        ram.load_at(Self::FONT_DATA_ADDR, &FONT_DATA[..]);
        ram.load_at(Self::PROG_ADDR, &rom);

        Ok(Self {
            pc: Self::PROG_ADDR,
            ram,
            regs: Registers::default(),
            gfx: Gfx::new(),
            stack: Stack::new(),
            delay_timer: 0,
            sound_timer: 0,
            keys: [false; 16],
        })
    }

    fn tick(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn emulate_cycle(&mut self) {
        self.tick();
        let opcode = self.fetch_opcode();
        debug!("Decoding opcode {:#0X} at pc={:#0X}", opcode, self.pc);

        match opcode & 0xF000 {
            0x0000 => {
                if opcode == 0x00E0 {
                    // Clear the screen
                    self.gfx.clear();
                    self.pc += 2;
                } else if opcode == 0x00EE {
                    // Return from subroutine
                    self.pc = self.stack.pop();
                    debug!("Returning from subroutine to {:#X}", self.pc);
                    self.pc += 2;
                } else {
                    // Call RCA1802 program
                    panic!("unimplemented opcode {:#x}", opcode);
                }
            }
            0x1000 => {
                // jump
                let addr = opcode & 0x0FFF;
                self.pc = addr;
            }
            0x2000 => {
                // Call subroutine
                let addr = opcode & 0x0FFF;
                debug!("Calling subroutine at {:#X}", addr);
                self.stack.push(self.pc);
                self.pc = addr;
            }
            0x3000 => {
                // Skip next instruction if VX == NN
                let reg = ((opcode & 0x0F00) >> 8) as u8;
                let value = (opcode & 0x00FF) as u8;
                if self.regs[reg] == value {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x4000 => {
                // Skip next instruction if VX == NN
                let reg = ((opcode & 0x0F00) >> 8) as u8;
                let value = (opcode & 0x00FF) as u8;
                if self.regs[reg] != value {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x5000 => {
                // Skip the next instruction if VX == VY
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let y = ((opcode & 0x00F0) >> 4) as u8;
                if self.regs[x] == self.regs[y] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x6000 => {
                // Set VX to NN
                let reg = ((opcode & 0x0F00) >> 8) as u8;
                let value = (opcode & 0x00FF) as u8;
                self.regs[reg] = value;
                self.pc += 2;
            }
            0x7000 => {
                // Add NN to VX (don't set carry flag)
                let reg = ((opcode & 0x0F00) >> 8) as u8;
                let value = (opcode & 0x00FF) as u8;
                self.regs[reg] = self.regs[reg].wrapping_add(value);
                self.pc += 2;
            }
            0x8000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let y = ((opcode & 0x00F0) >> 4) as u8;
                let op = (opcode & 0x000F) as u8;

                match op {
                    // Set Vx to Vy
                    0 => self.regs[x] = self.regs[y],
                    1 => self.regs[x] = self.regs[x] | self.regs[y],
                    2 => self.regs[x] = self.regs[x] & self.regs[y],
                    3 => self.regs[x] = self.regs[x] ^ self.regs[y],
                    4 => {
                        let (sum, overflow) = self.regs[x].overflowing_add(self.regs[y]);
                        self.regs[x] = sum;
                        self.regs.set_carry(overflow);
                    }
                    5 => {
                        let (diff, overflow) = self.regs[x].overflowing_sub(self.regs[y]);
                        self.regs[x] = diff;
                        self.regs.set_carry(overflow);
                    }
                    6 => {
                        let lsb = self.regs[x] & 0x01;
                        self.regs[x] = self.regs[y] >> 1;
                        if lsb == 1 {
                            self.regs.set_carry(true);
                        } else {
                            self.regs.set_carry(false);
                        }
                    }
                    7 => {
                        let (diff, overflow) = self.regs[y].overflowing_sub(self.regs[x]);
                        self.regs[x] = diff;
                        self.regs.set_carry(overflow);
                    }
                    0x0E => {
                        let msb = self.regs[x] & 0x80;
                        self.regs[x] = self.regs[y] << 1;
                        if msb == 1 {
                            self.regs.set_carry(true);
                        } else {
                            self.regs.set_carry(false);
                        }
                    }
                    _ => panic!("invalid opcode {:#x}", opcode),
                }
                self.pc += 2;
            }
            0x9000 => {
                // Skip the next instruction if VX != VY
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let y = ((opcode & 0x00F0) >> 4) as u8;
                if self.regs[x] != self.regs[y] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0xA000 => {
                let addr = opcode & 0x0FFF;
                self.regs.I = addr;
                self.pc += 2;
            },
            0xD000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let y = ((opcode & 0x00F0) >> 4) as u8;
                let n = (opcode & 0x000F) as u8;
                self.draw_sprite(self.regs[x], self.regs[y], n);
                self.pc += 2;
            }
            0xE000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let op = opcode & 0x00FF;

                match op {
                    0x9E => {
                        if self.keys[self.regs[x] as usize] {
                            debug!("Key {} pressed", self.regs[x]);
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        if !self.keys[self.regs[x] as usize] {
                            self.pc += 4;
                        } else {
                            debug!("Key {} pressed", self.regs[x]);
                            self.pc += 2;
                        }
                    }
                    _ => panic!("Unkown opcode {:#x}", opcode),
                }
            }
            0xF000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let op = opcode & 0x00FF;

                match op {
                    0x07 => {
                        self.regs[x] = self.delay_timer;
                    }
                    0x0A => {
                        if let Some(idx) = self.keys.iter().position(|v| *v) {
                            self.regs[x] = idx as u8;
                            self.pc += 2;
                        } else {
                            // do not increment PC: the program is effectively halted until a key
                            // is pressed.
                        }
                    }
                    0x15 => {
                        self.delay_timer = self.regs[x];
                    }
                    0x18 => {
                        self.sound_timer = self.regs[x];
                    }
                    0x1E => {
                        self.regs.I += self.regs[x] as u16;
                    }
                    0x29 => {
                        self.regs.I = Self::FONT_DATA_ADDR + self.regs[x] as u16 * 5;
                    }
                    0x33 => {
                        // BCD
                        let mut v = self.regs[x];
                        let units = v % 10;
                        v /= 10;
                        let tens = v % 10;
                        v /= 10;
                        let hundreds = v % 10;
                        self.ram[self.regs.I] = hundreds;
                        self.ram[self.regs.I + 1] = tens;
                        self.ram[self.regs.I + 2] = units;

                    }
                    0x55 => {
                        for i in 0..=x {
                            self.ram[self.regs.I] = self.regs[i];
                            self.regs.I += 1;
                        }
                    }
                    0x65 => {
                        for i in 0..=x {
                            self.regs[i] = self.ram[self.regs.I];
                            self.regs.I += 1;
                        }
                    }
                    _ => panic!("unknown opcode {:#x}", opcode),
                }
                self.pc += 2;
            }
            _ => panic!("unknown opcode {:#x}", opcode),
        }
    }

    pub fn gfx_buffer(&self) -> &[u8] {
        &self.gfx.0[..]
    }

    pub fn set_key(&mut self, key: u8, is_down: bool) {
        self.keys[key as usize] = is_down;
    }

    fn fetch_opcode(&self) -> u16 {
        ((self.ram[self.pc] as u16) << 8) | (self.ram[self.pc + 1] as u16)
    }

    /// Draw sprite at address I at coordinates (vx, vy) with height `n`
    fn draw_sprite(&mut self, vx: u8, vy: u8, n: u8) {
        self.gfx.draw_sprite(vx, vy, n, &self.ram.get_sprite(self.regs.I, n));
    }
}

struct Ram(Box<[u8]>);

impl Ram {
    /// Load the content of `data` into RAM at address `addr`.
    pub fn load_at(&mut self, addr: u16, data: &[u8]) {
        let addr = addr as usize;
        let data_size = data.len();
        let dest = &mut self.0[addr..addr + data_size];
        debug!("Writing {} bytes into ram", data.len());
        dest.copy_from_slice(data);
    }

    /// Return the data for the sprite at address `addr` with height `height`.
    pub fn get_sprite(&self, addr: u16, height: u8) -> &[u8] {
        &self.0[(addr as usize)..((addr + height as u16) as usize)]
    }
}

impl Default for Ram {
    fn default() -> Self {
        Self(vec![0u8; 4096].into_boxed_slice())
    }
}

impl std::ops::Index<u16> for Ram {
    type Output = u8;

    fn index(&self, idx: u16) -> &u8 {
        &self.0[idx as usize]
    }
}

impl std::ops::IndexMut<u16> for Ram {

    fn index_mut(&mut self, idx: u16) -> &mut u8 {
        &mut self.0[idx as usize]
    }
}

/// Holds general purpose registers
#[allow(non_snake_case)]
#[derive(Debug, Default)]
struct Registers {
    /// Address register
    I: u16,
    pub V0: u8,
    pub V1: u8,
    pub V2: u8,
    pub V3: u8,
    pub V4: u8,
    pub V5: u8,
    pub V6: u8,
    pub V7: u8,
    pub V8: u8,
    pub V9: u8,
    pub VA: u8,
    pub VB: u8,
    pub VC: u8,
    pub VD: u8,
    pub VE: u8,
    pub VF: u8,
}

impl Registers {
    pub fn set_carry(&mut self, flag: bool) {
        self.VF = if flag { 1 } else { 0 };
    }
}

impl std::ops::Index<u8> for Registers {
    type Output = u8;

    fn index(&self, idx: u8) -> &u8 {
        match idx {
            0x0 => &self.V0,
            0x1 => &self.V1,
            0x2 => &self.V2,
            0x3 => &self.V3,
            0x4 => &self.V4,
            0x5 => &self.V5,
            0x6 => &self.V6,
            0x7 => &self.V7,
            0x8 => &self.V8,
            0x9 => &self.V9,
            0xA => &self.VA,
            0xB => &self.VB,
            0xC => &self.VC,
            0xD => &self.VD,
            0xE => &self.VE,
            0xF => &self.VF,
            _ => panic!("Invalid register {}", idx),
        }
    }
}

impl std::ops::IndexMut<u8> for Registers {
    fn index_mut(&mut self, idx: u8) -> &mut u8 {
        match idx {
            0x0 => &mut self.V0,
            0x1 => &mut self.V1,
            0x2 => &mut self.V2,
            0x3 => &mut self.V3,
            0x4 => &mut self.V4,
            0x5 => &mut self.V5,
            0x6 => &mut self.V6,
            0x7 => &mut self.V7,
            0x8 => &mut self.V8,
            0x9 => &mut self.V9,
            0xA => &mut self.VA,
            0xB => &mut self.VB,
            0xC => &mut self.VC,
            0xD => &mut self.VD,
            0xE => &mut self.VE,
            0xF => &mut self.VF,
            _ => panic!("Invalid register {}", idx),
        }
    }
}

struct Gfx([u8; 64 * 32]);

impl Gfx {
    const W: u8 = 64;
    const H: u8 = 32;

    pub fn new() -> Self {
        Self([0u8; (Self::W as usize * Self::H as usize)])
    }

    pub fn clear(&mut self) {
        for v in self.0.iter_mut() {
            *v = 0;
        }
    }

    pub fn draw_sprite(&mut self, x: u8, y: u8, height: u8, data: &[u8]) {
        let x = x % Self::W;
        let y = y % Self::H;

        for dy in 0..height {
            let sprite_byte = data[dy as usize];
            for dx in 0..8 {
                let bit = sprite_byte & (0x80 >> dx);
                self.set(x + dx, y + dy, bit);
            }
        }
    }

    pub fn set(&mut self, x:u8, y: u8, v: u8) {
        if x < Self::W && y < Self::H {
            self.0[(y as u16 * Self::W as u16 + x as u16) as usize] ^= v;
        }
    }
}

struct Stack {
    st: [u16; 16],
    sp: u16,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            st: [0u16; 16],
            sp: 0,
        }
    }

    pub fn push(&mut self, v: u16) {
        assert!(self.sp < 16, "stack overflow");
        self.sp += 1;
        self.st[self.sp as usize] = v;
    }

    pub fn pop(&mut self) -> u16 {
        assert!(self.sp > 0, "stack underflow");
        let v = self.st[self.sp as usize];
        self.sp -= 1;
        v
    }
}

fn main() {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;

    env_logger::try_init().unwrap();

    let args = std::env::args().collect::<Vec<_>>();
    let rom = args.get(1).expect("missing rom file");
    info!("loading rom {}", rom);
    let mut chip8 = Chip8::new(rom).unwrap();

    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale: Scale::X32,
            scale_mode: ScaleMode::Stretch,
            topmost: false,
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    // window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (i, key) in KEYS.iter().enumerate() {
            chip8.set_key(i as u8, window.is_key_down(*key));
        }
        chip8.emulate_cycle();
        buffer.iter_mut().zip(chip8.gfx_buffer().iter())
            .for_each(|(b, v)| *b = if *v == 0 { 0u32} else { 0xFFFFFFFF });

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}

const FONT_DATA: [u8; 5 * 16] = [
0xF0, 0x90, 0x90, 0x90, 0xF0,
0x20, 0x60, 0x20, 0x20, 0x70,
0xF0, 0x10, 0xF0, 0x80, 0xF0,
0xF0, 0x10, 0xF0, 0x10, 0xF0,
0x90, 0x90, 0xF0, 0x10, 0x10,
0xF0, 0x80, 0xF0, 0x10, 0xF0,
0xF0, 0x80, 0xF0, 0x90, 0xF0,
0xF0, 0x10, 0x20, 0x40, 0x40,
0xF0, 0x90, 0xF0, 0x90, 0xF0,
0xF0, 0x90, 0xF0, 0x10, 0xF0,
0xF0, 0x90, 0xF0, 0x90, 0x90,
0xE0, 0x90, 0xE0, 0x90, 0xE0,
0xF0, 0x80, 0x80, 0x80, 0xF0,
0xE0, 0x90, 0x90, 0x90, 0xE0,
0xF0, 0x80, 0xF0, 0x80, 0xF0,
0xF0, 0x80, 0xF0, 0x80, 0x80,
];

const KEYS: [Key; 16] = [
    Key::Key1, Key::Key2, Key::Key3, Key::Q, Key::W, Key::E, Key::A, Key::S, Key::D, Key::X, Key::Z, Key::C, Key::Key4, Key::R, Key::F, Key::V
];
