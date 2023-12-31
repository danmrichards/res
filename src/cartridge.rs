use crate::{
    mapper::{Mapper, NROM},
    rom::Rom,
};

/// Represents the screen mirroring mode.
#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

/// Represents a NES cartridge.
pub struct Cartridge {
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    /// Creates a new Cartridge from the given raw ROM data.
    pub fn new(raw: &[u8]) -> Result<Cartridge, String> {
        let rom = match Rom::new(raw) {
            Ok(rom) => rom,
            Err(e) => return Err(e),
        };

        let mapper = rom.header.mapper();
        let cart = Cartridge {
            mapper: match mapper {
                0 => Box::new(NROM::new(rom)),
                _ => return Err(format!("Mapper {} is not supported", mapper)),
            },
        };

        Ok(cart)
    }

    /// Returns a byte from PRG ROM at the given address.
    pub fn read_prg(&self, addr: u16) -> u8 {
        self.mapper.read_prg(addr)
    }

    /// Writes a byte to PRG ROM at the given address.
    pub fn write_prg(&mut self, addr: u16, data: u8) {
        self.mapper.write_prg(addr, data)
    }

    /// Returns a byte from CHR ROM at the given address.
    pub fn read_chr(&self, addr: u16) -> u8 {
        self.mapper.read_chr(addr)
    }

    /// Writes a byte to CHR ROM at the given address.
    pub fn write_chr(&mut self, addr: u16, data: u8) {
        self.mapper.write_chr(addr, data)
    }

    /// Returns the Mirroring mode.
    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring()
    }
}
