use std::borrow::Borrow;

use crate::cartridge::Mirroring;

/// PPUBus abstracts a single location for interacting with vram and palette
/// memory.
pub struct PPUBus {
    /// Character (visuals) ROM.
    pub chr_rom: Vec<u8>,

    /// Internal reference to colour palettes.
    pub palette_table: [u8; 32],

    /// Video RAM.
    pub vram: [u8; 2048],

    pub mirroring: Mirroring,
}

pub trait Memory {
    fn write_data(&mut self, addr: u16, value: u8);
    fn read_data(&mut self, addr: u16) -> u8;
    fn bg_palette(&self, attribute_table: &[u8], col: usize, row: usize) -> [u8; 4];
    fn sprite_palette(&self, idx: u8) -> [u8; 4];
    fn read_chr_rom(&self, start: usize, end: usize) -> &[u8];
    fn read_palette_table(&self, idx: usize) -> u8;
    fn nametables(&self, addr: u16) -> (&[u8], &[u8]);
}

impl PPUBus {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPUBus {
            chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            mirroring,
        }
    }

    /// Horizontal:
    ///   [ A ] [ a ]
    ///   [ B ] [ b ]
    ///
    /// Vertical:
    ///   [ A ] [ B ]
    ///   [ a ] [ b ]
    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        // Mirror down 0x3000-0x3EFF to 0x2000 - 0x2EFF
        let mirrored_vram = addr & 0b1011111_1111111;

        // To VRAM vector.
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;

        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }
}

impl Memory for PPUBus {
    /// Writes data to appropriate location based on the address register.
    fn write_data(&mut self, addr: u16, value: u8) {
        match addr {
            0..=0x1FFF => println!("attempt to write to chr rom space {}", addr),
            0x2000..=0x2FFF => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),

            // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of
            // $3F00/$3F04/$3F08/$3F0C
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3F00) as usize] = value;
            }
            0x3F00..=0x3FFF => {
                self.palette_table[(addr - 0x3F00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    /// Retuns data from appropriate source based on the address register.
    fn read_data(&mut self, addr: u16) -> u8 {
        match addr {
            0..=0x1FFF => self.chr_rom[addr as usize],
            0x2000..=0x2FFF => self.vram[self.mirror_vram_addr(addr) as usize],
            0x3000..=0x3EFF => panic!(
                "addr space 0x3000..0x3EFF is not expected to be used, requested = {} ",
                addr
            ),
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize],
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    /// Returns the background palette for a specific column and row on screen.
    fn bg_palette(&self, attribute_table: &[u8], col: usize, row: usize) -> [u8; 4] {
        // Each background tile is one byte in the nametable space in VRAM.
        let attr_table_idx = row / 4 * 8 + col / 4;
        let attr = attribute_table[attr_table_idx];

        // A byte in an attribute table controls which palettes are used for 4x4
        // tile blocks or 32x32 pixels.
        //
        // A byte is split into four 2-bit blocks and each block is assigning a
        // background palette for four neighboring tiles.
        //
        // Determine which tile we're dealing with and match the appropriate part
        // of the byte.
        //
        // Example:
        //
        //  0b11011000 => 0b|11|01|10|00 => 11,01,10,00
        let palette_idx = match (col % 4 / 2, row % 4 / 2) {
            (0, 0) => attr & 0b11,
            (1, 0) => (attr >> 2) & 0b11,
            (0, 1) => (attr >> 4) & 0b11,
            (1, 1) => (attr >> 6) & 0b11,
            (_, _) => panic!("invalid palette index"),
        };

        let start: usize = 1 + (palette_idx as usize) * 4;
        [
            self.palette_table[0],
            self.palette_table[start],
            self.palette_table[start + 1],
            self.palette_table[start + 2],
        ]
    }

    /// Returns the sprite palette for a given index
    fn sprite_palette(&self, idx: u8) -> [u8; 4] {
        let start = 0x11 + (idx * 4) as usize;
        [
            0,
            self.palette_table[start],
            self.palette_table[start + 1],
            self.palette_table[start + 2],
        ]
    }

    // TODO(dr): Remove this.
    fn read_chr_rom(&self, start: usize, end: usize) -> &[u8] {
        self.chr_rom[start..=end].borrow()
    }

    // TODO(dr): Remove this.
    fn read_palette_table(&self, idx: usize) -> u8 {
        self.palette_table[idx]
    }

    // TODO(dr): Remove this.
    fn nametables(&self, addr: u16) -> (&[u8], &[u8]) {
        let (main_nametable, second_nametable) = match (&self.mirroring, addr) {
            (Mirroring::Vertical, 0x2000)
            | (Mirroring::Vertical, 0x2800)
            | (Mirroring::Horizontal, 0x2000)
            | (Mirroring::Horizontal, 0x2400) => (&self.vram[0..0x400], &self.vram[0x400..0x800]),
            (Mirroring::Vertical, 0x2400)
            | (Mirroring::Vertical, 0x2C00)
            | (Mirroring::Horizontal, 0x2800)
            | (Mirroring::Horizontal, 0x2C00) => (&self.vram[0x400..0x800], &self.vram[0..0x400]),
            (_, _) => {
                panic!("Not supported mirroring type {:?}", self.mirroring);
            }
        };
        (main_nametable, second_nametable)
    }
}
