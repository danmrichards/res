use super::Mapper;
use crate::{cartridge::Mirroring, rom::Rom};

/// NROM refers to the Nintendo cartridge boards NES-NROM-128, NES-NROM-256,
/// their HVC counterparts, and clone boards. The iNES format assigns mapper 0
/// to NROM.
pub struct NROM {
    rom: Rom,
    ram: Vec<u8>,
}

impl NROM {
    /// Returns an instantiated NROM.
    pub fn new(rom: Rom) -> Self {
        NROM {
            rom,
            ram: vec![0; 0x2000],
        }
    }
}

impl Mapper for NROM {
    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, mut addr: u16) -> u8 {
        if self.rom.prg.len() == 0x4000 && addr >= 0x4000 {
            // Mirror if needed.
            addr %= 0x4000;
        }
        self.rom.prg[addr as usize]
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
