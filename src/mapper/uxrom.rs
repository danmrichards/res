use super::Mapper;
use crate::{cartridge::Mirroring, rom::Rom, rom::PRG_PAGE_SIZE};

const FIXED_BANK_START: u16 = 0xC000;
const FIXED_BANK_END: u16 = 0xFFFF;
const PAGE_OFFSET_MASK: u16 = 0x3FFF;

/// UxROM refers to the Nintendo cartridge boards NES-UNROM, NES-UOROM,
/// HVC-UN1ROM their HVC counterparts, and clone boards.
pub struct Uxrom {
    rom: Rom,
    bank: usize,
}

impl Uxrom {
    pub fn new(rom: Rom) -> Self {
        Uxrom { rom, bank: 0 }
    }
}

impl Mapper for Uxrom {
    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            // 16 KB PRG ROM bank, fixed to the last bank
            FIXED_BANK_START..=FIXED_BANK_END => {
                let index = (self.rom.header.prg_size() - 1) * PRG_PAGE_SIZE
                    + (addr & PAGE_OFFSET_MASK) as usize;
                self.rom.prg[index]
            }

            // 16 KB switchable PRG ROM bank.
            _ => {
                let index = self.bank * PRG_PAGE_SIZE + (addr & PAGE_OFFSET_MASK) as usize;
                self.rom.prg[index]
            }
        }
    }

    /// Writes a byte to PRG ROM at the given address.
    fn write_prg(&mut self, addr: u16, data: u8) {
        // Writes in the range 0x8000-0xFFFF select the 16 KB PRG ROM bank.
        // (UNROM uses bits 2-0; UOROM uses bits 3-0).
        if let 0x8000..=0xFFFF = addr {
            self.bank = (data & 0xF) as usize;
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
