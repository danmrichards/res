/// Represents a sprite from OAM.
#[derive(Clone, Copy, Default, Debug)]
pub struct Sprite {
    pub id: u8,

    /// X & Y position of the top-left corner of the sprite.
    pub x: u8,
    pub y: u8,

    /// Tile index number.
    ///
    /// For 8x8 sprites, this is the tile number of this sprite within the
    /// pattern table selected in bit 3 of PPUCTRL ($2000).
    ///
    /// For 8x16 sprites, the PPU ignores the pattern table selection and selects
    /// a pattern table from bit 0 of this number.
    pub index: u8,

    /// Sprite attributes.
    ///
    /// 7     bit     0
    /// ------- -------
    /// | | | | | | | |
    /// | | | | | | + +- Palette (4 to 7) of sprite
    /// | | | + + + ---- Unimplemented (read 0)
    /// | | +----------- Priority (0: in front of background; 1: behind background)
    /// | +------------  Flip sprite horizontally
    /// +--------------- Flip sprite vertically
    pub attr: u8,
}
