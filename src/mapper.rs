mod nrom;
mod uxrom;

pub use nrom::Nrom;
pub use uxrom::Uxrom;

use crate::cartridge::Mirroring;

pub trait Mapper {
    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, addr: u16) -> u8;

    /// Writes a byte to PRG ROM at the given address.
    fn write_prg(&mut self, addr: u16, data: u8);

    /// Returns a byte from CHR ROM at the given address.
    fn read_chr(&self, addr: u16) -> u8;

    /// Writes a byte to CHR ROM at the given address.
    fn write_chr(&mut self, addr: u16, data: u8);

    /// Returns the Mirroring mode.
    fn mirroring(&self) -> Mirroring;
}
