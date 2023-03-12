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
}
