const INES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_PAGE_SIZE: usize = 16384;
const CHR_PAGE_SIZE: usize = 8192;

#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

// Represents a ROM in the iNES format.
//
// See: https://www.nesdev.org/wiki/INES
pub struct Rom {
    // Contains program code.
    pub prg: Vec<u8>,

    // Contains pattern tables and graphics.
    pub chr: Vec<u8>,

    // Mappers allow cartridge roms to define additional memory.
    pub mapper: u8,

    // Screen mirroring mode.
    pub screen_mirroring: Mirroring,
}

impl Rom {
    pub fn new(raw: &Vec<u8>) -> Result<Rom, String> {
        if &raw[0..4] != INES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        let mapper = (raw[7] & 0b11110000) | (raw[6] >> 4);

        let ines_ver = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err("NES2.0 format is not supported".to_string());
        }

        let four_screen = raw[6] & 0b1000 != 0;
        let vertical_mirroring = raw[6] & 0b1 != 0;
        let screen_mirroring = match (four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        // PRG is sized in 16kb units.
        let prg_size = raw[4] as usize * PRG_PAGE_SIZE;

        // CHR is sized in 8kb units.
        let chr_size = raw[5] as usize * CHR_PAGE_SIZE;

        let skip_trainer = raw[6] & 0b100 != 0;

        let prg_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_start = prg_start + prg_size;

        Ok(Rom {
            prg: raw[prg_start..(prg_start + prg_size)].to_vec(),
            chr: raw[chr_start..(chr_start + chr_size)].to_vec(),
            mapper,
            screen_mirroring,
        })
    }
}

pub mod test {

    use super::*;

    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        prg: Vec<u8>,
        chr: Vec<u8>,
    }

    fn create_rom(rom: TestRom) -> Vec<u8> {
        let mut result = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().map_or(0, |t| t.len())
                + rom.prg.len()
                + rom.chr.len(),
        );

        result.extend(&rom.header);
        if let Some(t) = rom.trainer {
            result.extend(t);
        }
        result.extend(&rom.prg);
        result.extend(&rom.chr);

        result
    }

    pub fn test_rom() -> Rom {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg: vec![1; 2 * PRG_PAGE_SIZE],
            chr: vec![2; 1 * CHR_PAGE_SIZE],
        });

        Rom::new(&test_rom).unwrap()
    }

    #[test]
    fn test() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg: vec![1; 2 * PRG_PAGE_SIZE],
            chr: vec![2; 1 * CHR_PAGE_SIZE],
        });

        let rom: Rom = Rom::new(&test_rom).unwrap();

        assert_eq!(rom.chr, vec!(2; 1 * CHR_PAGE_SIZE));
        assert_eq!(rom.prg, vec!(1; 2 * PRG_PAGE_SIZE));
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, Mirroring::Vertical);
    }

    #[test]
    fn test_with_trainer() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E,
                0x45,
                0x53,
                0x1A,
                0x02,
                0x01,
                0x31 | 0b100,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
            ],
            trainer: Some(vec![0; 512]),
            prg: vec![1; 2 * PRG_PAGE_SIZE],
            chr: vec![2; 1 * CHR_PAGE_SIZE],
        });

        let rom: Rom = Rom::new(&test_rom).unwrap();

        assert_eq!(rom.chr, vec!(2; 1 * CHR_PAGE_SIZE));
        assert_eq!(rom.prg, vec!(1; 2 * PRG_PAGE_SIZE));
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, Mirroring::Vertical);
    }

    #[test]
    fn test_nes2_is_not_supported() {
        let test_rom = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x31, 0x8, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg: vec![1; 1 * PRG_PAGE_SIZE],
            chr: vec![2; 1 * CHR_PAGE_SIZE],
        });
        let rom = Rom::new(&test_rom);
        match rom {
            Ok(_) => assert!(false, "should not load rom"),
            Err(str) => assert_eq!(str, "NES2.0 format is not supported"),
        }
    }
}
