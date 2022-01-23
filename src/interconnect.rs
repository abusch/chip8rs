use crate::gfx::Gfx;
use crate::ram::Ram;

/// Main "Bus" of the Chip-8 machine.
///
/// It coordinates access to the RAM, timers, keys, and display.
pub struct Interconnect {
    pub ram: Ram,
    pub gfx: Gfx,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub keys: [bool; 16],
}

impl Interconnect {
    pub fn tick(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    /// Fetch the 2-byte long instruction at address `pc`.
    pub fn fetch_opcode(&self, pc: u16) -> u16 {
        ((self.ram[pc] as u16) << 8) | (self.ram[pc + 1] as u16)
    }

    /// Draw sprite located at address `addr` at coordinates (vx, vy) with height `n`
    pub fn draw_sprite(&mut self, addr: u16, vx: u8, vy: u8, n: u8) -> bool {
        self.gfx
            .draw_sprite(vx, vy, n, self.ram.get_sprite(addr, n))
    }
}
