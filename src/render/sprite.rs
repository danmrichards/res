const FLIP_HORIZONTAL: u8 = 0b01000000;
const FLIP_VERTICAL: u8 = 0b10000000;
const SPRITE_PALETTE: u8 = 0b00000011;
const BEHIND_BACKGROUND: u8 = 0b00100000;

// Represents a sprite from OAM.
pub struct Sprite {
    // X & Y position of the top-left corner of the sprite.
    pub x: usize,
    pub y: usize,

    // Tile index number.
    //
    // For 8x8 sprites, this is the tile number of this sprite within the
    // pattern table selected in bit 3 of PPUCTRL ($2000).
    //
    // For 8x16 sprites, the PPU ignores the pattern table selection and selects
    // a pattern table from bit 0 of this number.
    pub index: u16,

    // Sprite attributes.
    //
    // 7     bit     0
    // ------- -------
    // | | | | | | | |
    // | | | | | | + +- Palette (4 to 7) of sprite
    // | | | + + + ---- Unimplemented (read 0)
    // | | +----------- Priority (0: in front of background; 1: behind background)
    // | +------------  Flip sprite horizontally
    // +--------------- Flip sprite vertically
    pub attr: u8,
}

impl Sprite {
    // Returns an instantiated sprite.
    pub fn new(oam_data: &[u8]) -> Self {
        Sprite {
            x: oam_data[3] as usize,
            y: oam_data[0] as usize,
            index: oam_data[1] as u16,
            attr: oam_data[2],
        }
    }

    // Returns the palette index of the sprite.
    pub fn palette_index(&self) -> u8 {
        self.attr & SPRITE_PALETTE
    }

    // Returns true if the sprite should be flipped horizontally.
    pub fn flip_horizontal(&self) -> bool {
        self.attr & FLIP_HORIZONTAL == FLIP_HORIZONTAL
    }

    // Returns true if the sprite should be flipped vertically.
    pub fn flip_vertical(&self) -> bool {
        self.attr & FLIP_VERTICAL == FLIP_VERTICAL
    }

    // Returns true if the sprite is behind the background.
    pub fn behind_background(&self) -> bool {
        self.attr & BEHIND_BACKGROUND == BEHIND_BACKGROUND
    }
}
