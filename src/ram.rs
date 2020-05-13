use log::debug;

/// the RAM of the Chip-8 machine.
///
/// It consists of 4096 bytes that can be individually addressed using 16-bit addresses.
pub struct Ram(Box<[u8]>);

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
