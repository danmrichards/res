pub mod registers;

mod frame;
mod palette;
mod sprite;
mod tile;

use crate::bus::Memory;
use registers::addr::Addr;
use registers::control::Control;
use registers::mask::Mask;
use registers::scroll::Scroll;
use registers::status::Status;

use self::frame::Frame;
use self::palette::Rgb;
use self::palette::COLOUR_PALETTE;
use self::sprite::Sprite;
use self::tile::Tile;

const SCREEN_SIZE: usize = 0x3C0;
const OAM_SIZE: usize = 0x100;
const OAM2_SIZE: usize = 0x8;

/// Represents a rectangle viewport.
struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    /// Returns an instantiated Rect.
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
}

/// Represents the NES PPU.
pub struct NESPPU<'rcall> {
    /// Bus to allow PPU to interact with RAM/ROM.
    bus: Box<dyn Memory>,
    open_bus: u8,
    open_bus_timer: u32,

    /// Object attribute memory (sprites).
    pub oam_addr: u8,
    pub oam_data: [u8; OAM_SIZE],
    oam2_data: [Sprite; OAM2_SIZE],
    clearing_oam: bool,
    sprite_0_rendering: bool,
    sprite_count: usize,
    fg_lo_shift: [u8; OAM2_SIZE],
    fg_hi_shift: [u8; OAM2_SIZE],

    /// Registers.
    pub addr: Addr,
    pub ctrl: Control,
    pub mask: Mask,
    pub scroll: Scroll,
    pub status: Status,

    /// Is the NMI interrupt set?
    pub nmi_interrupt: Option<bool>,

    /// Buffer for data read from previous request.
    buf: u8,
    xfine: u8,

    /// Current picture scan line
    scanline: i32,

    /// Current cycle.
    cycle: usize,

    next_tile: Tile,
    bg_lo_shift: u16,
    bg_hi_shift: u16,
    bg_attr_lo_shift: u16,
    bg_attr_hi_shift: u16,

    /// Number of frames rendered by the PPU.
    frame_count: u128,
    odd_frame: bool,

    /// Current frame.
    frame: Frame,

    /// Callback to render frame.
    render_callback: Box<dyn FnMut(&[u8]) + 'rcall>,
}

pub trait PPU {
    fn write_addr(&mut self, value: u8);
    fn write_ctrl(&mut self, value: u8);
    fn write_mask(&mut self, value: u8);
    fn write_scroll(&mut self, value: u8);
    fn write_data(&mut self, value: u8);
    fn write_oam_addr(&mut self, value: u8);
    fn write_oam_data(&mut self, value: u8);
    fn write_oam_dma(&mut self, value: &[u8; 256]);
    fn read_data(&mut self) -> u8;
    fn read_status(&mut self) -> u8;
    fn read_oam_data(&self) -> u8;
    fn read_frame_count(&self) -> u128;
}

impl<'a> NESPPU<'a> {
    /// Returns an instantiated PPU.
    pub fn new<'rcall, F>(bus: Box<dyn Memory>, render_callback: F) -> NESPPU<'rcall>
    where
        F: FnMut(&[u8]) + 'rcall,
    {
        NESPPU {
            bus,
            open_bus: 0,
            open_bus_timer: 0,
            oam_addr: 0,
            oam_data: [0; OAM_SIZE],
            oam2_data: [Sprite::default(); OAM2_SIZE],
            clearing_oam: false,
            sprite_0_rendering: false,
            sprite_count: 0,
            fg_lo_shift: [0; OAM2_SIZE],
            fg_hi_shift: [0; OAM2_SIZE],
            buf: 0,
            xfine: 0,
            addr: Addr::new(),
            ctrl: Control::new(),
            mask: Mask::new(),
            scroll: Scroll::new(),
            status: Status::new(),
            scanline: 0,
            cycle: 0,
            next_tile: Tile::default(),
            bg_lo_shift: 0,
            bg_hi_shift: 0,
            bg_attr_lo_shift: 0,
            bg_attr_hi_shift: 0,
            nmi_interrupt: None,
            frame_count: 0,
            odd_frame: false,
            frame: Frame::new(),
            render_callback: Box::from(render_callback),
        }
    }

    /// Increment the VRAM address based on the control register status.
    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    /// Returns true if a frame has been completed.
    pub fn clock(&mut self) {
        // Update the open bus timer
        self.update_open_bus();

        // Every odd frame on the first scanline, the first cycle is skipped if
        // background rendering is enabled. A flag is updated every frame.
        if self.odd_frame && self.scanline == 0 && self.cycle == 0 && self.rendering_enabled() {
            self.cycle = 1;
        }

        // To not have to write self. every time
        let cycle = self.cycle;
        let scanline = self.scanline;

        // Pre render scanline
        if scanline == -1 && cycle == 1 {
            // Clear NMI and reset status register
            self.nmi_interrupt = None;
            self.status.set_sprite_zero_hit(false);
            self.status.set_sprite_overflow(false);
            self.status.set_vblank_status(false);

            // Clear sprite shifters
            self.fg_lo_shift.fill(0);
            self.fg_hi_shift.fill(0);
        }

        if scanline < 240 && self.rendering_enabled() {
            // TODO(dr): Process scanline.
        }

        // Set NMI if enabled on cycle 241
        if scanline == 241 && cycle == 1 {
            self.status.set_vblank_status(true);
            if self.ctrl.vblank_nmi() {
                self.nmi_interrupt = Some(true)
            }

            // A new frame is done rendering
            self.frame_count = self.frame_count.wrapping_add(1);

            // Render in window (in this case, using SDL2)
            (self.render_callback)(self.frame.pixels());
        }

        // Calculate the pixel color
        if (0..240).contains(&scanline) && (1..257).contains(&cycle) {
            let (bg_pixel, bg_palette) = self.get_bg_pixel_info();

            // Hack to fix random sprite colors on left of first scanline.
            let (fg_pixel, fg_palette, fg_priority) = match scanline != 0 {
                true => self.get_fg_pixel_info(),
                false => (0, 0, 0),
            };

            // Pixel priority logic
            let (pixel, palette) = match bg_pixel {
                // Both foreground and background are 0, result is 0
                0 if fg_pixel == 0 => (0, 0),
                // Only background is 0, output foreground
                0 if fg_pixel > 0 => (fg_pixel, fg_palette),
                // Only foreground is 0, output background
                1..=3 if fg_pixel == 0 => (bg_pixel, bg_palette),
                // Both are non zero
                _ => {
                    // Collision is possible
                    self.update_sprite_zero_hit();

                    // The result is choosen based on the sprite priority
                    // attribute.
                    if fg_priority != 0 {
                        (fg_pixel, fg_palette)
                    } else {
                        (bg_pixel, bg_palette)
                    }
                }
            };

            // Get the color from palette RAM
            let colour = self.get_colour(palette, pixel);

            self.frame.set_pixel(cycle - 1, scanline as usize, colour);
        }

        // Update cycle count
        self.cycle += 1;

        // Last cycle
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;

            // Last scanline
            if self.scanline > 260 {
                self.scanline = -1;
                self.odd_frame = !self.odd_frame;
            }
        }
    }

    /// Refresh open bus latch timer
    fn update_open_bus(&mut self) {
        match self.open_bus_timer > 0 {
            true => self.open_bus_timer -= 1,
            false => self.open_bus = 0,
        }
    }

    /// Returns if the rendering is enabled or not
    fn rendering_enabled(&self) -> bool {
        self.mask.show_sprites() | self.mask.show_background()
    }

    /// Returns pixel value and palette index of current background pixel.
    fn get_bg_pixel_info(&self) -> (u8, u8) {
        // Return early if we're not rendering the background.
        if !self.mask.show_background()
            || !(self.mask.leftmost_8pxl_background() || self.cycle >= 9)
        {
            (0, 0);
        }

        let mux = 0x8000 >> self.xfine;

        let lo_pixel = ((self.bg_lo_shift & mux) != 0) as u8;
        let hi_pixel = ((self.bg_hi_shift & mux) != 0) as u8;
        let bg_pixel = (hi_pixel << 1) | lo_pixel;

        let lo_pal = ((self.bg_attr_lo_shift & mux) != 0) as u8;
        let hi_pal = ((self.bg_attr_hi_shift & mux) != 0) as u8;
        let bg_palette = (hi_pal << 1) | lo_pal;

        return (bg_pixel, bg_palette);
    }

    /// Returns pixel value, palette index and attribute byte of current
    /// foreground pixel.
    fn get_fg_pixel_info(&mut self) -> (u8, u8, u8) {
        if self.mask.show_sprites() && (self.mask.leftmost_8pxl_sprite() || self.cycle >= 9) {
            self.sprite_0_rendering = false;
            for i in 0..self.sprite_count {
                if self.oam2_data[i].x != 0 {
                    continue;
                }

                let lo_pixel = ((self.fg_lo_shift[i] & 0x80) != 0) as u8;
                let hi_pixel = ((self.fg_hi_shift[i] & 0x80) != 0) as u8;
                let fg_pixel = (hi_pixel << 1) | lo_pixel;

                let fg_palette = (self.oam2_data[i].attr & 0x3) + 0x4;
                let fg_priority = ((self.oam2_data[i].attr & 0x20) == 0) as u8;

                if fg_pixel != 0 {
                    // Set a flag if it is sprite 0
                    if self.oam2_data[i].index == 0 {
                        self.sprite_0_rendering = true;
                    }
                    return (fg_pixel, fg_palette, fg_priority);
                }
            }
        }

        (0, 0, 0)
    }

    /// Update the sprite 0 hit flag.
    fn update_sprite_zero_hit(&mut self) {
        // Sprite 0 hit is a collision between a non 0 sprite pixel and bg pixel
        // To be possible, we have to be drawing a sprite 0 pixel and both
        // sprite and background rendering has to be enabled.
        if !self.sprite_0_rendering || !self.mask.show_background() || !self.mask.show_sprites() {
            return;
        }

        // If either bg or sprite left most pixels are disabled, don't check
        // first 8 pixels.
        if !(self.mask.leftmost_8pxl_background() | self.mask.leftmost_8pxl_sprite()) {
            if (9..256).contains(&self.cycle) {
                self.status.set_sprite_zero_hit(true);
            }
        } else if (1..256).contains(&self.cycle) {
            self.status.set_sprite_zero_hit(true);
        }
    }

    /// Returns the RBG value of the pixel with greyscale and colour emphasis
    /// applied.
    fn get_colour(&mut self, palette: u8, pixel: u8) -> Rgb {
        let index = self
            .bus
            .read_data(0x3F00 + ((palette as u16) << 2) + pixel as u16)
            & self.mask.grayscale_mask();

        let c = COLOUR_PALETTE[(index as usize) & 0x3F];

        match self.mask.colour_emphasis_enabled() {
            false => c,
            true => {
                let (r, g, b) = self.mask.emphasise();
                Rgb(
                    (c.0 as f64 * r) as u8,
                    (c.1 as f64 * g) as u8,
                    (c.2 as f64 * b) as u8,
                )
            }
        }
    }
}

impl PPU for NESPPU<'_> {
    /// Writes value to the address register.
    fn write_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    /// Writes to the control register.
    fn write_ctrl(&mut self, value: u8) {
        let start_nmi = self.ctrl.vblank_nmi();

        self.ctrl.update(value);

        if !start_nmi && self.ctrl.vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(true);
        }
    }

    /// Writes to the mask register.
    fn write_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    /// Writes to the scroll register.
    fn write_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }

    /// Returns the PPU status register and resets VBLANK + addr.
    fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.reset_vblank_status();
        self.addr.reset();
        self.scroll.reset_latch();
        data
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    /// Returns number of frames rendered.
    fn read_frame_count(&self) -> u128 {
        self.frame_count
    }

    fn write_data(&mut self, data: u8) {
        self.bus.write_data(self.addr.get(), data);
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        // Reading data takes 2 reads to get the data. The first read put the
        // data in a buffer amd the second read puts the buffer data on the bus.
        let result = self.buf;
        self.buf = self.bus.read_data(addr);
        result
    }
}

#[cfg(test)]
pub mod test {
    use crate::bus::PPUBus;
    use crate::cartridge::Mirroring;

    use super::*;

    /// Returns an instatiated PPU with an empty ROM loaded.
    pub fn new_empty_rom_ppu() -> NESPPU<'static> {
        let bus = PPUBus::new(vec![0; 2048], Mirroring::Horizontal);
        NESPPU::new(Box::new(bus), |_| {})
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.bus.read_data(0x0305), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x0305, 0x66);

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x01ff, 0x66);
        ppu.bus.write_data(0x0200, 0x77);

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0b100);
        ppu.bus.write_data(0x01ff, 0x66);
        ppu.bus.write_data(0x01ff + 32, 0x77);
        ppu.bus.write_data(0x01ff + 64, 0x88);

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_addr(0x24);
        ppu.write_addr(0x05);

        ppu.write_data(0x66);

        ppu.write_addr(0x28);
        ppu.write_addr(0x05);

        ppu.write_data(0x77);

        ppu.write_addr(0x20);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);

        ppu.write_addr(0x2C);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x77);
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let bus = PPUBus::new(vec![0; 2048], Mirroring::Vertical);
        let ppu = NESPPU::new(Box::new(bus), |_| {});

        ppu.write_addr(0x20);
        ppu.write_addr(0x05);

        ppu.write_data(0x66);

        ppu.write_addr(0x2C);
        ppu.write_addr(0x05);

        ppu.write_data(0x77);

        ppu.write_addr(0x28);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);

        ppu.write_addr(0x24);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = new_empty_rom_ppu();
        ppu.bus.write_data(0x0305, 0x66);

        ppu.write_addr(0x21);
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x0305, 0x66);

        ppu.write_addr(0x63);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = new_empty_rom_ppu();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = new_empty_rom_ppu();
        ppu.write_oam_addr(0x10);
        ppu.write_oam_data(0x66);
        ppu.write_oam_data(0x77);

        ppu.write_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = new_empty_rom_ppu();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_oam_addr(0xF);
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x77);

        ppu.write_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x66);
    }
}
