use super::Mapper;
use crate::{cartridge::Mirroring, rom::Rom};

/// NROM refers to the Nintendo cartridge boards NES-NROM-128, NES-NROM-256,
/// their HVC counterparts, and clone boards. The iNES format assigns mapper 0
/// to NROM.
pub struct Nrom {
    rom: Rom,
    ram: Vec<u8>,
}

impl Nrom {
    /// Returns an instantiated NROM.
    pub fn new(rom: Rom) -> Self {
        Nrom {
            rom,
            ram: vec![0; 0x2000],
        }
    }

    /// Returns the PRG ROM mask used for PRG ROM bank switching.
    fn prg_mask(&self) -> u16 {
        if self.rom.header.prg_size() > 1 {
            0x7FFF
        } else {
            0x3FFF
        }
    }
}

impl Mapper for Nrom {
    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            // Special case for "Family Basic".
            0x6000..=0x7FFF => self.ram[(addr & 0x1FFF) as usize],

            _ => self.rom.prg[(addr & self.prg_mask()) as usize],
        }
    }

    /// Writes a byte to PRG ROM at the given address.
    fn write_prg(&mut self, addr: u16, data: u8) {
        if let 0x6000..=0x7FFF = addr {
            self.ram[(addr & 0x1FFF) as usize] = data;
        }
    }

    /// Returns a byte from CHR ROM at the given address.
    fn read_chr(&self, addr: u16) -> u8 {
        self.rom.chr[addr as usize]
    }

    /// Writes a byte to CHR ROM at the given address.
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.rom.header.chr_size() == 0 {
            self.rom.chr[addr as usize] = data;
        }
    }

    /// Returns the Mirroring mode.
    fn mirroring(&self) -> Mirroring {
        self.rom.header.mirroring()
    }
}
