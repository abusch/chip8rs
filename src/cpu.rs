use log::debug;

use crate::config;
use crate::Interconnect;

/// The CPU of the Chip-8 machine.
///
/// It decodes and executes instructions fetched from RAM (via the `Interconnect`), and maintains a
/// set of registers and a stack.
/// TODO: The stack should really be a pointer to some address in the RAM.
pub struct Cpu {
    pc: u16,
    regs: Registers,
    stack: Stack,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            pc: config::PROG_ADDR,
            regs: Registers::default(),
            stack: Stack::new(),
        }
    }

    pub fn emulate_cycle(&mut self, interconnect: &mut Interconnect) {
        let opcode = interconnect.fetch_opcode(self.pc);
        debug!("Decoding opcode {:#0X} at pc={:#0X}", opcode, self.pc);

        match opcode & 0xF000 {
            0x0000 => {
                if opcode == 0x00E0 {
                    // Clear the screen
                    interconnect.gfx.clear();
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
            }
            0xD000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let y = ((opcode & 0x00F0) >> 4) as u8;
                let n = (opcode & 0x000F) as u8;
                interconnect.draw_sprite(self.regs.I, self.regs[x], self.regs[y], n);
                self.pc += 2;
            }
            0xE000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let op = opcode & 0x00FF;

                match op {
                    0x9E => {
                        if interconnect.keys[self.regs[x] as usize] {
                            debug!("Key {} pressed", self.regs[x]);
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        if !interconnect.keys[self.regs[x] as usize] {
                            self.pc += 4;
                        } else {
                            debug!("Key {} pressed", self.regs[x]);
                            self.pc += 2;
                        }
                    }
                    _ => panic!("Unkown opcode {:#x}", opcode),
                }
            }
            // Misc
            0xF000 => {
                let x = ((opcode & 0x0F00) >> 8) as u8;
                let op = opcode & 0x00FF;

                match op {
                    0x07 => {
                        self.regs[x] = interconnect.delay_timer;
                    }
                    0x0A => {
                        if let Some(idx) = interconnect.keys.iter().position(|v| *v) {
                            self.regs[x] = idx as u8;
                            self.pc += 2;
                        } else {
                            // do not increment PC: the program is effectively halted until a key
                            // is pressed.
                        }
                    }
                    0x15 => {
                        interconnect.delay_timer = self.regs[x];
                    }
                    0x18 => {
                        interconnect.sound_timer = self.regs[x];
                    }
                    0x1E => {
                        self.regs.I += self.regs[x] as u16;
                    }
                    0x29 => {
                        self.regs.I = config::FONT_DATA_ADDR + self.regs[x] as u16 * 5;
                    }
                    0x33 => {
                        // BCD
                        let mut v = self.regs[x];
                        let units = v % 10;
                        v /= 10;
                        let tens = v % 10;
                        v /= 10;
                        let hundreds = v % 10;
                        interconnect.ram[self.regs.I] = hundreds;
                        interconnect.ram[self.regs.I + 1] = tens;
                        interconnect.ram[self.regs.I + 2] = units;
                    }
                    0x55 => {
                        for i in 0..=x {
                            interconnect.ram[self.regs.I] = self.regs[i];
                            self.regs.I += 1;
                        }
                    }
                    0x65 => {
                        for i in 0..=x {
                            self.regs[i] = interconnect.ram[self.regs.I];
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
