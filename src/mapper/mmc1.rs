use super::Mapper;
use crate::{cartridge::Mirroring, rom::Rom};

/// MMC1 is a memory mapper used in Nintendo's SxROM and NES-EVENT Game Pak
/// boards.
pub struct MMC1 {
    rom: Rom,

    chr_lo: u8,
    chr_hi: u8,
    chr_8k: u8,
    prg_lo: u8,
    prg_hi: u8,
    prg_32k: u8,

    // Control register.
    //
    // 4bit0
    // -----
    // CPPMM
    // |||||
    // |||++- Mirroring (0: one-screen, lower bank; 1: one-screen, upper bank;
    // |||               2: vertical; 3: horizontal)
    // |++--- PRG ROM bank mode (0, 1: switch 32 KB at $8000, ignoring low bit of bank number;
    // |                         2: fix first bank at $8000 and switch 16 KB bank at $C000;
    // |                         3: fix last bank at $C000 and switch 16 KB bank at $8000)
    // +----- CHR ROM bank mode (0: switch 8 KB at a time; 1: switch two separate 4 KB banks)
    control: u8,

    // Load register.
    //
    // 7  bit  0
    // ---- ----
    // Rxxx xxxD
    // |       |
    // |       +- Data bit to be shifted into shift register, LSB first
    // +--------- A write with bit set will reset shift register
    //             and write Control with (Control OR $0C),
    //             locking PRG ROM at $C000-$FFFF to the last bank.
    load: u8,

    count: u8,
    ram: Vec<u8>,
    mirroring: Mirroring,
}

impl MMC1 {
    pub fn new(rom: Rom) -> Self {
        let prg_hi = (rom.header.prg_size() - 1) as u8;

        MMC1 {
            rom,

            chr_lo: 0,
            chr_hi: 0,
            chr_8k: 0,
            prg_lo: 0,
            prg_hi,
            prg_32k: 0,

            control: 0x0C,
            count: 0,
            load: 0,

            ram: vec![0; 0x2000],
            mirroring: Mirroring::Vertical,
        }
    }
}

impl Mapper for MMC1 {
    /// Returns a byte from PRG ROM at the given address.
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            // 8 KB PRG RAM bank.
            0x6000..=0x7FFF => self.ram[(addr & 0x1FFF) as usize],

            // 16 KB PRG ROM bank.
            0x8000..=0xFFFF => {
                // Switch PRG ROM bank based on the control register.
                let index = if self.control & 0x8 != 0 {
                    if addr >= 0x8000 && addr <= 0xBFFF {
                        self.prg_lo as usize * 0x4000 + (addr & 0x3FFF) as usize
                    } else {
                        self.prg_hi as usize * 0x4000 + (addr & 0x3FFF) as usize
                    }
                } else {
                    self.prg_32k as usize * 0x8000 + (addr & 0x7FFF) as usize
                };

                self.rom.prg[index]
            }
            _ => 0,
        }
    }

    /// Writes a byte to PRG ROM at the given address.
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            // 8 KB PRG RAM bank.
            0x6000..=0x7FFF => self.ram[(addr & 0x1FFF) as usize] = data,

            // 16 KB PRG ROM bank.
            0x8000..=0xFFFF => {
                if data & 0x80 != 0 {
                    self.control |= 0x0C;
                    self.count = 0;
                    self.load = 0;
                } else {
                    self.load |= (data & 0x1) << self.count;
                    self.count += 1;

                    if self.count == 5 {
                        let target = (addr >> 13) & 0x3;
                        let chr_4k_mode = self.control & 0x10 != 0;
                        match target {
                            0 => {
                                self.control = self.load & 0x1F;
                                self.mirroring = match self.control & 0x3 {
                                    0 => Mirroring::SingleScreenLo,
                                    1 => Mirroring::SingleScreenHi,
                                    2 => Mirroring::Vertical,
                                    _ => Mirroring::Horizontal,
                                };
                            }
                            1 => {
                                if chr_4k_mode {
                                    self.chr_lo = self.load & 0x1F;
                                } else {
                                    self.chr_8k = (self.load & 0x1E) >> 1;
                                }
                            }
                            2 => {
                                if chr_4k_mode {
                                    self.chr_hi = self.load & 0x1F;
                                }
                            }
                            _ => {
                                let prg_mode = (self.control >> 2) & 0x3;

                                match prg_mode {
                                    0 | 1 => self.prg_32k = (self.load & 0xE) >> 1,
                                    2 => {
                                        self.prg_lo = 0;
                                        self.prg_hi = self.load & 0xF;
                                    }
                                    _ => {
                                        self.prg_lo = self.load & 0xF;
                                        self.prg_hi = (self.rom.header.prg_size() - 1) as u8;
                                    }
                                }
                            }
                        }

                        self.count = 0;
                        self.load = 0;
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns a byte from CHR ROM at the given address.
    fn read_chr(&self, addr: u16) -> u8 {
        if self.rom.header.chr_size() == 0 {
            return self.rom.chr[addr as usize];
        }

        // Check if the CHR ROM bank mode is 8 KB or 4 KB.
        let index = if self.control & 0x10 != 0 {
            match addr {
                0x0000..=0x0FFF => self.chr_lo as usize * 0x1000 + (addr & 0xFFF) as usize,
                0x1000..=0x1FFF => self.chr_hi as usize * 0x1000 + (addr & 0xFFF) as usize,
                _ => 0,
            }
        } else {
            self.chr_8k as usize * 0x2000 + (addr & 0x1FFF) as usize
        };

        self.rom.chr[index]
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
