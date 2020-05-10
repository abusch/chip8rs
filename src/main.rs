use std::path::Path;

struct Chip8 {
    pc: u16,
    ram: Ram,
    regs: Registers,
    gfx: Gfx,
    stack: Stack,
    delay_timer: u8,
    sound_timer: u8,
}

impl Chip8 {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let rom = std::fs::read(path)?;
        let mut ram = Ram::default();
        ram.load(&rom);

        Ok(Self {
            pc: 0x200,
            ram,
            regs: Registers::default(),
            gfx: Gfx::new(),
            stack: Stack::new(),
            delay_timer: 0,
            sound_timer: 0,
        })
    }

    pub fn emulate_cycle(&mut self) {
        let opcode = self.fetch_opcode();
        println!("Decoding opcode {:#x} at pc={:#x}", opcode, self.pc);

        match opcode & 0xF000 {
            0x0000 => {
                if opcode == 0x00E0 {
                    // Clear the screen
                    self.gfx.clear();
                    self.pc += 2;
                } else if opcode == 0x00EE {
                    // Return from subroutine
                    self.pc = self.stack.pop();
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
            0xF000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let op = opcode & 0x00FF;

                match op {
                    0x07 => {
                        self.regs[x] = self.delay_timer;
                        self.pc += 2;
                    }
                    0x0A => {
                        todo!()
                    }
                    0x15 => {
                        self.delay_timer = self.regs[x];
                        self.pc += 2;
                    }
                    0x18 => {
                        self.sound_timer = self.regs[x];
                        self.pc += 2;
                    }
                    0x1E => {
                        self.regs.I += self.regs[x] as u16;
                        self.pc += 2;
                    }
                    0x29 => {

                    }
                    _ => panic!("unknown opcode {:#x}", opcode),
                }
            }
            _ => panic!("unknown opcode {:#x}", opcode),
        }
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
    pub fn load(&mut self, rom: &[u8]) {
        let rom_size = rom.len();
        let rom_space = &mut self.0[0x0200..0x0200 + rom_size];
        println!("Writing {} bytes into ram", rom.len());
        rom_space.copy_from_slice(rom);
        // rom_space.write_all(rom)
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
    let args = std::env::args().collect::<Vec<_>>();
    let rom = args.get(1).expect("missing rom file");
    println!("loading rom {}", rom);
    let mut chip8 = Chip8::new(rom).unwrap();

    loop {
        chip8.emulate_cycle();
    }
}
