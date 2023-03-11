const GRAYSCALE: u8 = 0b00000001;
const LEFTMOST_8PXL_BACKGROUND: u8 = 0b00000010;
const LEFTMOST_8PXL_SPRITE: u8 = 0b00000100;
const SHOW_BACKGROUND: u8 = 0b00001000;
const SHOW_SPRITES: u8 = 0b00010000;
const EMPHASISE_RED: u8 = 0b00100000;
const EMPHASISE_GREEN: u8 = 0b01000000;
const EMPHASISE_BLUE: u8 = 0b10000000;

/// Represents the PPU mask register.
pub struct Mask {
    /// 7  bit  0
    /// ---- ----
    /// B G R s b M m G
    /// | | | | | | | |
    /// | | | | | | | +- Grayscale (0: normal color, 1: produce a grayscale display)
    /// | | | | | | +--- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    /// | | | | | +----- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    /// | | | | +------- 1: Show background
    /// | | | +--------- 1: Show sprites
    /// | | +----------- Emphasize red (green on PAL/Dendy)
    /// | +------------- Emphasize green (red on PAL/Dendy)
    /// +--------------- Emphasize blue
    bits: u8,
}

impl Mask {
    /// Returns a new mask register.
    pub fn new() -> Self {
        Mask { bits: 0b00000000 }
    }

    /// Returns the grayscale mask value
    pub fn grayscale_mask(&self) -> u8 {
        match (self.bits & GRAYSCALE) == GRAYSCALE {
            true => 0x30,
            false => 0xFF,
        }
    }

    /// Returns true if the left most 8 pixel background is enabled.
    pub fn leftmost_8pxl_background(&self) -> bool {
        (self.bits & LEFTMOST_8PXL_BACKGROUND) == LEFTMOST_8PXL_BACKGROUND
    }

    /// Returns true if the left most 8 pixel sprite is enabled.
    pub fn leftmost_8pxl_sprite(&self) -> bool {
        (self.bits & LEFTMOST_8PXL_SPRITE) == LEFTMOST_8PXL_SPRITE
    }

    /// Returns true if the background should be shown.
    pub fn show_background(&self) -> bool {
        (self.bits & SHOW_BACKGROUND) == SHOW_BACKGROUND
    }

    /// Returns true if sprites should be shown.
    pub fn show_sprites(&self) -> bool {
        (self.bits & SHOW_SPRITES) == SHOW_SPRITES
    }

    /// Returns the current colour emphasis.
    pub fn emphasise(&self) -> (f64, f64, f64) {
        let mut r = 1.0;
        let mut g = 1.0;
        let mut b = 1.0;

        if (self.bits & EMPHASISE_RED) == EMPHASISE_RED {
            g = 0.75;
            b = 0.75;
        }
        if (self.bits & EMPHASISE_GREEN) == EMPHASISE_GREEN {
            r = 0.75;
            b = 0.75;
        }
        if (self.bits & EMPHASISE_BLUE) == EMPHASISE_BLUE {
            r = 0.75;
            b = 0.75;
        }

        (r, g, b)
    }

    /// Returns true if one of the color emphasis bits is set.
    pub fn colour_emphasis_enabled(&self) -> bool {
        self.bits & (EMPHASISE_RED | EMPHASISE_GREEN | EMPHASISE_BLUE) != 0
    }

    /// Updates the state of the register.
    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}
