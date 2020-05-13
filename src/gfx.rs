const W: u8 = 64;
const H: u8 = 32;

/// Represents the display of the Chip-8 machine.
///
/// It consists of 64x32 1-bit pixels.
pub struct Gfx {
    buf: [u8; (W as usize * H as usize)],
    pub dirty: bool,
}

impl Gfx {
    pub fn new() -> Self {
        Self {
            buf: [0u8; (W as usize * H as usize)],
            dirty: true,
        }
    }

    pub fn clear(&mut self) {
        for v in self.buf.iter_mut() {
            *v = 0;
        }
        self.dirty = true;
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
        self.dirty = true;
    }

    pub fn set(&mut self, x: u8, y: u8, v: u8) {
        if x < W && y < H {
            self.buf[(y as u16 * W as u16 + x as u16) as usize] ^= v;
        }
    }

    pub fn get_frame(&mut self) -> &[u8] {
        self.dirty = false;
        &self.buf[..]
    }
}
