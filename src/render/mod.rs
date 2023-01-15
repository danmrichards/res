pub mod frame;
pub mod palette;

use crate::ppu::NESPPU;
use frame::Frame;

const SCREEN_SIZE: usize = 0x3C0;

// Returns the background palette for a specific column and row on screen.
fn bg_palette(ppu: &NESPPU, col: usize, row: usize) -> [u8; 4] {
    // Each background tile is one byte in the nametable space in VRAM.
    let attr_table_idx = row / 4 * 8 + col / 4;

    // NOTE: Harcoded to the first name table.
    let attr = ppu.vram[SCREEN_SIZE + attr_table_idx];

    // A byte in an attribute table controls which palettes are used for 4x4
    // tile blocks or 32x32 pixels.
    //
    // A byte is split into four 2-bit blocks and each block is assigning a
    // background palette for four neighboring tiles.
    //
    // Determine which tile we're dealing with and match the appropriate part
    // of the byte.
    //
    // Example:
    //
    //  0b11011000 => 0b|11|01|10|00 => 11,01,10,00
    let palette_idx = match (col % 4 / 2, row % 4 / 2) {
        (0, 0) => attr & 0b11,
        (1, 0) => (attr >> 2) & 0b11,
        (0, 1) => (attr >> 4) & 0b11,
        (1, 1) => (attr >> 6) & 0b11,
        (_, _) => panic!("invalid palette index"),
    };

    let start: usize = 1 + (palette_idx as usize) * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

// Renders a screen of pixels to the frame based on PPU state.
pub fn render(ppu: &NESPPU, frame: &mut Frame) {
    let bank = ppu.ctrl.bgrnd_pattern_addr();

    // NES screen is made up of 960 tiles (32x30).
    for i in 0..SCREEN_SIZE {
        let tile = ppu.vram[i] as u16;
        let col = i % 32;
        let row = i / 32;
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];

        // Lookup the background colour palette for this column and row.
        let palette = bg_palette(ppu, col, row);

        // Each background tile on screen is 8x8 pixels.
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;

                // A background tile can have up to 4 colours.
                let rgb = match value {
                    0 => palette::COLOUR_PALETTE[ppu.palette_table[0] as usize],
                    1 => palette::COLOUR_PALETTE[palette[1] as usize],
                    2 => palette::COLOUR_PALETTE[palette[2] as usize],
                    3 => palette::COLOUR_PALETTE[palette[3] as usize],
                    _ => panic!("invalid tile index"),
                };
                frame.set_pixel(col * 8 + x, row * 8 + y, rgb)
            }
        }
    }
}
