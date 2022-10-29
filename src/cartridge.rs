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
    pub screen_mirror: Mirroring,
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
        let screen_mirror = match (four_screen, vertical_mirroring) {
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
            screen_mirror,
        })
    }
}
