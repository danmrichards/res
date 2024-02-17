use crate::cartridge::Mirroring;

const INES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
pub const PRG_PAGE_SIZE: usize = 16384;
pub const CHR_PAGE_SIZE: usize = 8192;

/// Represents the iNES header.
///
/// 0-3     Constant $4E $45 $53 $1A (ASCII "NES" followed by MS-DOS end-of-file)
/// 4       Size of PRG ROM in 16 KB units
/// 5       Size of CHR ROM in 8 KB units (value 0 means the board uses CHR RAM)
/// 6       Flags 6 – Mapper, mirroring, battery, trainer
/// 7       Flags 7 – Mapper, VS/Playchoice, NES 2.0
/// 8       Flags 8 – PRG-RAM size (rarely used extension)
/// 9       Flags 9 – TV system (rarely used extension)
/// 10      Flags 10 – TV system, PRG-RAM presence (unofficial, rarely used extension)
/// 11-15   Unused padding (should be filled with zero, but some rippers put their name across bytes 7-15)
pub struct Header {
    /// Size of PRG ROM in 16 KB units
    prg_size: u8,

    /// Size of CHR ROM in 8 KB units (value 0 means the board uses CHR RAM)
    chr_size: u8,

    /// Flags 6 – Mapper, mirroring, battery, trainer
    ///
    /// 00110001
    /// ||||||||
    /// |||||||+- Mirroring: 0: horizontal (vertical arrangement) (CIRAM A10 = PPU A11)
    /// |||||||              1: vertical (horizontal arrangement) (CIRAM A10 = PPU A10)
    /// ||||||+-- 1: Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
    /// |||||+--- 1: 512-byte trainer at $7000-$71FF (stored before PRG data)
    /// ||||+---- 1: Ignore mirroring control or above mirroring bit; instead provide four-screen VRAM
    /// ++++----- Lower nybble of mapper number
    flags_6: u8,

    /// Flags 7 – Mapper, VS/Playchoice, NES 2.0
    ///
    /// 76543210
    /// ||||||||
    /// |||||||+- VS Unisystem
    /// ||||||+-- PlayChoice-10 (8 KB of Hint Screen data stored after CHR data)
    /// ||||++--- If equal to 2, flags 8-15 are in NES 2.0 format
    /// ++++----- Upper nybble of mapper number
    flags_7: u8,

    /// Flags 8 – PRG-RAM size (rarely used extension)
    ///
    /// 76543210
    /// ||||||||
    /// ++++++++- PRG RAM size
    flags_8: u8,

    /// Flags 9 – TV system (rarely used extension)
    ///
    /// 76543210
    /// ||||||||
    /// |||||||+- TV system (0: NTSC; 1: PAL)
    /// +++++++-- Reserved, set to zero
    flags_9: u8,

    /// Flags 10 – TV system, PRG-RAM presence (unofficial, rarely used extension)
    ///
    /// 76543210
    ///   ||  ||
    ///   ||  ++- TV system (0: NTSC; 2: PAL; 1/3: dual compatible)
    ///   |+----- PRG RAM ($6000-$7FFF) (0: present; 1: not present)
    ///   +------ 0: Board has no bus conflicts; 1: Board has bus conflict
    flags_10: u8,
}

impl Header {
    /// Returns the mapper number.
    pub fn mapper(&self) -> u8 {
        (self.flags_7 & 0xF0) | (self.flags_6 >> 4)
    }

    /// Returns true if the ROM provides four-screen VRAM.
    pub fn four_screen(&self) -> bool {
        self.flags_6 & 0x8 != 0
    }

    pub fn mirroring(&self) -> Mirroring {
        if self.four_screen() {
            Mirroring::FourScreen
        } else if self.flags_6 & 0x1 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        }
    }

    /// Returns the size of the PRG ROM in bytes.
    pub fn prg_size(&self) -> usize {
        self.prg_size as usize
    }

    /// Returns the size of the CHR ROM in bytes.
    pub fn chr_size(&self) -> usize {
        self.chr_size as usize
    }

    /// Returns true if the ROM contains a trainer.
    pub fn skip_trainer(&self) -> bool {
        self.flags_6 & 0x4 != 0
    }

    /// Returns the iNES version.
    fn ines_version(&self) -> u8 {
        (self.flags_7 >> 2) & 0x3
    }

    /// Creates a new header with default values.
    fn from_bytes(bytes: &[u8]) -> Header {
        Header {
            prg_size: bytes[4],
            chr_size: bytes[5],
            flags_6: bytes[6],
            flags_7: bytes[7],
            flags_8: bytes[8],
            flags_9: bytes[9],
            flags_10: bytes[10],
        }
    }
}

/// Represents a ROM in the iNES format.
///
/// See: https://www.nesdev.org/wiki/INES
pub struct Rom {
    /// The ROM header in iNES format.
    pub header: Header,

    /// Contains program code.
    pub prg: Vec<u8>,

    /// Contains pattern tables and graphics.
    pub chr: Vec<u8>,
}

impl Rom {
    pub fn new(raw: &[u8]) -> Result<Rom, String> {
        if raw[0..4] != INES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        let header = Header::from_bytes(raw);
        if header.ines_version() != 0 {
            return Err("NES2.0 format is not supported".to_string());
        }

        // PRG is sized in 16kb units.
        let prg_size = header.prg_size() * PRG_PAGE_SIZE;

        // CHR is sized in 8kb units.
        let chr_size = header.chr_size() * CHR_PAGE_SIZE;

        let prg_start = 16 + if header.skip_trainer() { 512 } else { 0 };
        let chr_start = prg_start + prg_size;

        let prg = raw[prg_start..(prg_start + prg_size)].to_vec();
        let chr = if header.chr_size() > 0 {
            raw[chr_start..(chr_start + chr_size)].to_vec()
        } else {
            vec![0; CHR_PAGE_SIZE]
        };

        Ok(Rom { header, prg, chr })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    const HEADER_TRAINER_DISABLED: u8 = 0b00110000;
    const HEADER_TRAINER_ENABLED: u8 = 0b00110100;
    const HEADER_NES_2_0: u8 = 0b00001000;

    /// Creates a new test ROM with given values.
    pub fn test_rom(
        prg_size: usize,
        prg: Vec<u8>,
        chr_size: usize,
        chr: Vec<u8>,
        trainer: Option<Vec<u8>>,
        flags_7: Option<u8>,
        mirroring: Option<Mirroring>,
    ) -> Result<Rom, String> {
        // Zero-pad PRG ROM up to the 16KB page size.
        let mut prg_rom = prg.clone();
        prg_rom.resize(prg_size * PRG_PAGE_SIZE, 0);

        // Zero-pad CHR ROM up to the 8KB page size.
        let mut chr_rom = chr.clone();
        chr_rom.resize(chr_size * CHR_PAGE_SIZE, 0);

        // Set the trainer byte in flags_6 if one is provided.
        let mut flags_6 = HEADER_TRAINER_DISABLED;
        if trainer.is_some() {
            flags_6 = HEADER_TRAINER_ENABLED;
        }

        // Set the mirroring bit in flags_6 if one is provided.
        if let Some(m) = mirroring {
            match m {
                Mirroring::Horizontal => {}
                Mirroring::Vertical => flags_6 ^= 0x1,
                Mirroring::FourScreen => flags_6 ^= 0x8,
            }
        }

        let mut header_bytes = INES_TAG.to_vec();
        header_bytes.append(&mut vec![
            prg_size as u8,
            chr_size as u8,
            flags_6,
            flags_7.unwrap_or(0),
            00,
            00,
            00,
            00,
            00,
            00,
            00,
            00,
        ]);

        let mut rom_bytes = Vec::with_capacity(
            header_bytes.len()
                + trainer.as_ref().map_or(0, |t| t.len())
                + prg_rom.len()
                + chr_rom.len(),
        );

        rom_bytes.extend(&header_bytes);
        if let Some(t) = trainer {
            rom_bytes.extend(t);
        }
        rom_bytes.extend(prg_rom);
        rom_bytes.extend(chr_rom);

        Rom::new(&rom_bytes)
    }

    #[test]
    fn test() {
        let prg_size = 1;
        let chr_size = 1;
        let rom = test_rom(
            prg_size,
            vec![0xA9, 0x05],
            chr_size,
            vec![0x00, 0x00],
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(rom.prg[0..2], vec![0xA9, 0x05]);
        assert_eq!(rom.prg.len(), prg_size * PRG_PAGE_SIZE);
        assert_eq!(rom.chr[0..2], vec![0x00, 0x00]);
        assert_eq!(rom.chr.len(), chr_size * CHR_PAGE_SIZE);
        assert_eq!(rom.header.mapper(), 3);
        assert_eq!(rom.header.mirroring(), Mirroring::Horizontal);
    }

    #[test]
    fn test_with_trainer() {
        let prg_size = 1;
        let chr_size = 1;
        let rom = test_rom(
            prg_size,
            vec![0xA9, 0x05],
            chr_size,
            vec![0x00, 0x00],
            Some(vec![0; 512]),
            None,
            None,
        )
        .unwrap();

        assert_eq!(rom.prg[0..2], vec![0xA9, 0x05]);
        assert_eq!(rom.prg.len(), prg_size * PRG_PAGE_SIZE);
        assert_eq!(rom.chr[0..2], vec![0x00, 0x00]);
        assert_eq!(rom.chr.len(), chr_size * CHR_PAGE_SIZE);
        assert_eq!(rom.header.mapper(), 3);
        assert_eq!(rom.header.mirroring(), Mirroring::Horizontal);
    }

    #[test]
    fn test_nes2_is_not_supported() {
        let rom = test_rom(
            1,
            vec![0xA9, 0x05],
            1,
            vec![0x00, 0x00],
            None,
            Some(HEADER_NES_2_0),
            None,
        );

        match rom {
            Ok(_) => unreachable!("should not load rom"),
            Err(str) => assert_eq!(str, "NES2.0 format is not supported"),
        }
    }
}
