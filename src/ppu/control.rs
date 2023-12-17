const NMI_ENABLED: u8 = 0b10000000;
const MASTER_SLAVE: u8 = 0b01000000;
const SPRITE_SIZE: u8 = 0b00100000;
const BG_ADDRESS: u8 = 0b00010000;
const SPRITE_ADDRESS: u8 = 0b00001000;
const VRAM_INCREMENT: u8 = 0b00000100;
const NAMETABLE_V: u8 = 0b00000010;
const NAMETABLE_H: u8 = 0b00000001;

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
    pub fn vram_addr_increment(&self) -> u16 {
        if self.bits & VRAM_INCREMENT != VRAM_INCREMENT {
            1
        } else {
            32
        }
    }

    /// Returns true if the PPU control is set to allow generation of a VBLANK
    /// interrupt.
    pub fn nmi_enabled(&self) -> bool {
        self.bits & NMI_ENABLED == NMI_ENABLED
    }

    /// Returns the address of the CHR ROM bank to use for background tiles.
    pub fn bgrnd_pattern_addr(&self) -> u16 {
        if self.bits & BG_ADDRESS != BG_ADDRESS {
            0
        } else {
            0x1000
        }
    }

    /// Returns the address of the CHR ROM bank to use for sprite tiles.
    pub fn sprite_pattern_addr(&self) -> u16 {
        if self.bits & SPRITE_ADDRESS != SPRITE_ADDRESS {
            0
        } else {
            0x1000
        }
    }

    /// Returns the sprite size flag value.
    pub fn sprite_size(&self) -> bool {
        self.bits & SPRITE_SIZE == SPRITE_SIZE
    }

    /// Returns the nametable H flag value
    pub fn nta_h(&self) -> bool {
        self.bits & NAMETABLE_H == NAMETABLE_H
    }

    /// Returns the nametable V flag value
    pub fn nta_v(&self) -> bool {
        self.bits & NAMETABLE_V == NAMETABLE_V
    }

    /// Sets the register to data.
    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}
