const NAMETABLE1: u8 = 0b00000001;
const NAMETABLE2: u8 = 0b00000010;
const VRAM_ADD_INCREMENT: u8 = 0b00000100;
const SPRITE_PATTERN_ADDR: u8 = 0b00001000;
const BACKROUND_PATTERN_ADDR: u8 = 0b00010000;
const SPRITE_SIZE: u8 = 0b00100000;
const MASTER_SLAVE_SELECT: u8 = 0b01000000;
const GENERATE_NMI: u8 = 0b10000000;

// Represents the PPU control register.
pub struct Control {
    // 7     bit     0
    // ------- -------
    // V P H B S I N N
    // | | | | | | | |
    // | | | | | | + +- Base nametable address
    // | | | | | |      (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // | | | | | +----- VRAM address increment per CPU read/write of PPUDATA
    // | | | | |        (0: add 1, going across; 1: add 32, going down)
    // | | | | +------- Sprite pattern table address for 8x8 sprites
    // | | | |          (0: $0000; 1: $1000; ignored in 8x16 mode)
    // | | | +--------- Background pattern table address (0: $0000; 1: $1000)
    // | | +----------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
    // | +------------- PPU master/slave select
    // |                (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------------- Generate an NMI at the start of the
    //                  vertical blanking interval (0: off; 1: on)
    bits: u8
}

impl Control {
    // Returns an instantiated control register.
    pub fn new() -> Self {
        Control {
            bits: 0b00000000,
        }
    }

    // Returns the amount to increment the VRAM addr by. 
    pub fn vram_addr_increment(&self) -> u8 {
        if self.bits & VRAM_ADD_INCREMENT != 0 {
            1
        } else {
            32
        }
    }
 
    // Sets the register to data.
    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}