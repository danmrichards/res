const NAMETABLE1: u8 = 0b00000001;
const NAMETABLE2: u8 = 0b00000010;
const VRAM_ADD_INCREMENT: u8 = 0b00000100;
const SPRITE_PATTERN_ADDR: u8 = 0b00001000;
const BACKROUND_PATTERN_ADDR: u8 = 0b00010000;
const SPRITE_SIZE: u8 = 0b00100000;
const MASTER_SLAVE_SELECT: u8 = 0b01000000;
const GENERATE_NMI: u8 = 0b10000000;

/// Represents the PPU control register.
pub struct Control {
    /// 7     bit     0
    /// ------- -------
    /// V P H B S I N N
    /// | | | | | | | |
    /// | | | | | | + +- Base nametable address
    /// | | | | | |      (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    /// | | | | | +----- VRAM address increment per CPU read/write of PPUDATA
    /// | | | | |        (0: add 1, going across; 1: add 32, going down)
    /// | | | | +------- Sprite pattern table address for 8x8 sprites
    /// | | | |          (0: $0000; 1: $1000; ignored in 8x16 mode)
    /// | | | +--------- Background pattern table address (0: $0000; 1: $1000)
    /// | | +----------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
    /// | +------------- PPU master/slave select
    /// |                (0: read backdrop from EXT pins; 1: output color on EXT pins)
    /// +--------------- Generate an NMI at the start of the
    ///                  vertical blanking interval (0: off; 1: on)
    bits: u8,
}

impl Control {
    /// Returns an instantiated control register.
    pub fn new() -> Self {
        Control { bits: 0b00000000 }
    }

    /// Returns the amount to increment the VRAM addr by.
    pub fn vram_addr_increment(&self) -> u8 {
        if self.bits & VRAM_ADD_INCREMENT != VRAM_ADD_INCREMENT {
            1
        } else {
            32
        }
    }

    /// Returns true if the PPU control is set to allow generation of a VBLANK
    /// interrupt.
    pub fn vblank_nmi(&self) -> bool {
        return self.bits & GENERATE_NMI == GENERATE_NMI;
    }

    /// Returns the address of the CHR ROM bank to use for background tiles.
    pub fn bgrnd_pattern_addr(&self) -> u16 {
        if self.bits & BACKROUND_PATTERN_ADDR != BACKROUND_PATTERN_ADDR {
            0
        } else {
            0x1000
        }
    }

    /// Returns the address of the CHR ROM bank to use for sprite tiles.
    pub fn sprite_pattern_addr(&self) -> u16 {
        if self.bits & SPRITE_PATTERN_ADDR != SPRITE_PATTERN_ADDR {
            0
        } else {
            0x1000
        }
    }

    /// Returns the address of the current nametable.
    pub fn nametable_addr(&self) -> u16 {
        match self.bits & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!("not possible"),
        }
    }

    /// Sets the register to data.
    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}
