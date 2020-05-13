const W: u8 = 64;
const H: u8 = 32;

/// Represents the display of the Chip-8 machine.
///
/// It consists of 64x32 1-bit pixels.
pub struct Gfx([u8; (W as usize * H as usize)]);

impl Gfx {
    pub fn new() -> Self {
        Self([0u8; (W as usize * H as usize)])
    }

    pub fn clear(&mut self) {
        for v in self.0.iter_mut() {
            *v = 0;
        }
    }

    pub fn draw_sprite(&mut self, x: u8, y: u8, height: u8, data: &[u8]) {
        let x = x % W;
        let y = y % H;

        for dy in 0..height {
            let sprite_byte = data[dy as usize];
            for dx in 0..8 {
                let bit = sprite_byte & (0x80 >> dx);
                self.set(x + dx, y + dy, bit);
            }
        }
    }

    pub fn set(&mut self, x: u8, y: u8, v: u8) {
        if x < W && y < H {
            self.0[(y as u16 * W as u16 + x as u16) as usize] ^= v;
        }
    }

    pub fn get_frame(&self) -> &[u8] {
        &self.0[..]
    }
}
