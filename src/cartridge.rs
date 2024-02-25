use crate::{
    mapper::{Mapper, Nrom, Uxrom, MMC1},
    rom::Rom,
};

/// Represents the screen mirroring mode.
#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    SingleScreenLo,
    SingleScreenHi,
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
                0 => Box::new(Nrom::new(rom)),
                1 => Box::new(MMC1::new(rom)),
                2 => Box::new(Uxrom::new(rom)),
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::rom::tests::test_rom;

    /// Creates a new Cartridge from the given PRG ROM data.
    pub fn test_cartridge(prg: Vec<u8>, mirroring: Option<Mirroring>) -> Result<Cartridge, String> {
        let rom = test_rom(1, prg, 1, vec![], None, None, mirroring).unwrap();

        Ok(Cartridge {
            mapper: Box::new(Nrom::new(rom)),
        })
    }

    #[test]
    fn test_new_cartridge() {
        let prg = vec![0; 16384];
        let cartridge = test_cartridge(prg.clone(), None).unwrap();
        assert_eq!(cartridge.read_prg(0), prg[0]);
    }

    #[test]
    fn test_read_prg() {
        let prg = vec![0; 16384];
        let cartridge = test_cartridge(prg.clone(), None).unwrap();
        assert_eq!(cartridge.read_prg(0), prg[0]);
    }

    #[test]
    fn test_write_prg() {
        let prg = vec![0; 16384];
        let mut cartridge = test_cartridge(prg.clone(), None).unwrap();
        cartridge.write_prg(0x6000, 1);
        assert_eq!(cartridge.read_prg(0x6000), 1);
    }

    #[test]
    fn test_read_chr() {
        let cartridge = test_cartridge(vec![0; 16384], None).unwrap();
        assert_eq!(cartridge.read_chr(0), 0);
    }

    #[test]
    fn test_mirroring() {
        let prg = vec![0; 16384];
        let cartridge = test_cartridge(prg.clone(), None).unwrap();
        assert_eq!(cartridge.mirroring(), Mirroring::Horizontal);
    }
}
