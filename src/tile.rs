use crate::render::frame::Frame;
use crate::render::palette;

pub fn show_tile(chr_rom: &Vec<u8>, bank: usize, tile_n: usize) -> Frame {
   assert!(bank <= 1);

   let mut frame = Frame::new();
   let bank = (bank * 0x1000) as usize;

   let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

   for y in 0..=7 {
       let mut upper = tile[y];
       let mut lower = tile[y + 8];

       for x in (0..=7).rev() {
           let value = (1 & upper) << 1 | (1 & lower);
           upper = upper >> 1;
           lower = lower >> 1;
           let rgb = match value {
               0 => palette::COLOUR_PALETTE[0x01],
               1 => palette::COLOUR_PALETTE[0x23],
               2 => palette::COLOUR_PALETTE[0x27],
               3 => palette::COLOUR_PALETTE[0x30],
               _ => panic!("can't be"),
           };
           frame.set_pixel(x, y, rgb)
       }
   }

   frame
}
