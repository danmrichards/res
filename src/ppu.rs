mod control;
mod frame;
mod mask;
mod palette;
mod scroll;
mod sprite;
mod status;
mod tile;

use crate::bus::Memory;
use control::Control;
use mask::Mask;
use scroll::Scroll;
use status::Status;

use self::frame::Frame;
use self::palette::Rgb;
use self::palette::COLOUR_PALETTE;
use self::sprite::Sprite;
use self::tile::Tile;

const OAM_SIZE: usize = 0x100;
const OAM2_SIZE: usize = 0x8;

type RenderFn<'rcall> = Box<dyn FnMut(&[u8]) + 'rcall>;

/// Represents the NES PPU.
pub struct NesPpu<'rcall> {
    /// Bus to allow PPU to interact with RAM/ROM.
    bus: Box<dyn Memory>,
    open_bus: u8,
    open_bus_timer: u32,

    /// Object attribute memory (sprites).
    oam_addr: u8,
    oam_data: [u8; OAM_SIZE],
    oam2_data: [Sprite; OAM2_SIZE],
    clearing_oam: bool,
    sprite_0_rendering: bool,
    sprite_count: usize,
    fg_lo_shift: [u8; OAM2_SIZE],
    fg_hi_shift: [u8; OAM2_SIZE],

    /// Registers.
    ctrl: Control,
    mask: Mask,
    scroll: Scroll,
    status: Status,

    /// Is the NMI interrupt set?
    pub nmi_interrupt: Option<bool>,

    /// Buffer for data read from previous request.
    buf: u8,
    addr_toggle: bool,
    v_addr: Scroll,
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
    render_callback: RenderFn<'rcall>,
}

pub trait Ppu {
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
    fn read_oam_data(&mut self) -> u8;
    fn read_frame_count(&self) -> u128;
}

impl<'a> NesPpu<'a> {
    /// Returns an instantiated PPU.
    pub fn new<'rcall, F>(bus: Box<dyn Memory>, render_callback: F) -> NesPpu<'rcall>
    where
        F: FnMut(&[u8]) + 'rcall,
    {
        NesPpu {
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
            addr_toggle: false,
            v_addr: Scroll::new(),
            xfine: 0,
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
        let new_addr = self
            .v_addr
            .raw()
            .wrapping_add(self.ctrl.vram_addr_increment());
        self.v_addr.set_raw(new_addr);
    }

    /// Poll the NMI flag set by the Ppu
    pub fn poll_nmi(&mut self) -> bool {
        self.nmi_interrupt.take().is_some()
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

        // Pre render scanline
        if self.scanline == -1 && self.cycle == 1 {
            // Clear NMI and reset status register
            self.nmi_interrupt = None;
            self.status.set_sprite_zero_hit(false);
            self.status.set_sprite_overflow(false);
            self.status.set_vblank_status(false);

            // Clear sprite shifters
            self.fg_lo_shift.fill(0);
            self.fg_hi_shift.fill(0);
        }

        if self.scanline < 240 && self.rendering_enabled() {
            self.render_scanline()
        }

        // Set NMI if enabled on cycle 241
        if self.scanline == 241 && self.cycle == 1 {
            self.status.set_vblank_status(true);
            if self.ctrl.nmi_enabled() {
                self.nmi_interrupt = Some(true)
            }

            self.frame_count = self.frame_count.wrapping_add(1);

            (self.render_callback)(self.frame.pixels());
        }

        // Calculate the pixel color
        if (0..240).contains(&self.scanline) && (1..257).contains(&self.cycle) {
            let (bg_pixel, bg_palette) = self.get_bg_pixel_info();

            // Hack to fix random sprite colors on left of first scanline.
            let (fg_pixel, fg_palette, fg_priority) = match self.scanline != 0 {
                true => self.get_fg_pixel_info(),
                false => (0, 0, 0),
            };

            // Pixel priority logic.
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

            self.frame
                .set_pixel(self.cycle - 1, self.scanline as usize, colour);
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

    /// Refresh open bus latch value
    pub fn refresh_open_bus(&mut self, data: u8) -> u8 {
        self.open_bus = data;
        self.open_bus_timer = 7777;
        data
    }

    /// Returns if the rendering is enabled or not
    fn rendering_enabled(&self) -> bool {
        self.mask.show_sprites() | self.mask.show_background()
    }

    /// Returns pixel value and palette index of current background pixel.
    fn get_bg_pixel_info(&self) -> (u8, u8) {
        if self.mask.show_background() && (self.mask.leftmost_8pxl_background() || self.cycle >= 9)
        {
            let mux = 0x8000 >> self.xfine;

            let lo_pixel = ((self.bg_lo_shift & mux) != 0) as u8;
            let hi_pixel = ((self.bg_hi_shift & mux) != 0) as u8;
            let bg_pixel = (hi_pixel << 1) | lo_pixel;

            let lo_pal = ((self.bg_attr_lo_shift & mux) != 0) as u8;
            let hi_pal = ((self.bg_attr_hi_shift & mux) != 0) as u8;
            let bg_palette = (hi_pal << 1) | lo_pal;

            return (bg_pixel, bg_palette);
        }

        (0, 0)
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

    /// Process the current cycle of a rendering scanline.
    fn render_scanline(&mut self) {
        // Update scroll on prerender scanline
        if self.scanline == -1 && self.cycle == 304 && self.mask.show_background() {
            self.v_addr = self.scroll;
        }

        // Background
        self.render_scanline_background();

        // Sprites
        self.render_scanline_sprites();
    }

    /// Renders the background for the current scanline.
    fn render_scanline_background(&mut self) {
        if (2..258).contains(&self.cycle) || (321..338).contains(&self.cycle) {
            // Update bg shifters
            self.shift_bg();

            // Background operations repeat every 8 cycles
            match (self.cycle - 1) % 8 {
                0 => {
                    self.load_next_tile();

                    // At the address is the id of the pattern to draw
                    let vaddr = self.v_addr.tile_addr();
                    self.next_tile.id = self.bus.read_data(vaddr);
                }
                2 => {
                    // Get the address of the tile attribute
                    let vaddr = self.v_addr.tile_attr_addr();
                    self.next_tile.attr = self.bus.read_data(vaddr);

                    // Attribute byte: BRBL TRTL
                    // BR: Bottom right metatile
                    // BL: Bottom left metatile
                    // TR: Top right metatile
                    // TL: Top left metatile

                    // Bottom part of the nametable?
                    if self.v_addr.ycoarse() & 0x2 != 0 {
                        // If so shift by 4
                        self.next_tile.attr >>= 4;
                    }
                    // Right part of the nametable?
                    if self.v_addr.xcoarse() & 0x2 != 0 {
                        // If so shift by 2
                        self.next_tile.attr >>= 2;
                    }
                    // Attribute is only two bits
                    self.next_tile.attr &= 0x3;
                }
                4 => {
                    // The pixel value are divided in two bitplanes.
                    // The bitplanes are 8 consecutive bytes in memory.
                    // So, the high and low bitplanes are 8 bytes apart
                    //
                    // Two bitplanes represent one background tile
                    // 0 1 1 0 0 1 2 0  =  0 1 1 0 0 1 1 0  +  0 0 0 0 0 0 1 0
                    // 0 0 0 0 0 1 2 0  =  0 0 0 0 0 1 1 0  +  0 0 0 0 0 0 1 0
                    // 0 0 0 0 0 1 2 0  =  0 0 0 0 0 1 1 0  +  0 0 0 0 0 0 1 0
                    // 0 1 0 0 0 0 2 0  =  0 1 0 0 0 0 1 0  +  0 0 0 0 0 0 1 0
                    // 0 1 1 0 0 0 1 0  =  0 1 1 0 0 0 0 0  +  0 0 0 0 0 0 1 0
                    // 0 0 0 0 0 1 2 0  =  0 0 0 0 0 1 1 0  +  0 0 0 0 0 0 1 0
                    // 0 1 1 0 0 0 1 0  =  0 1 1 0 0 0 0 0  +  0 0 0 0 0 0 1 0
                    // 0 1 1 0 0 1 2 0  =  0 1 1 0 0 1 1 0  +  0 0 0 0 0 0 1 0

                    let vaddr = self.ctrl.bgrnd_pattern_addr()
                        + ((self.next_tile.id as u16) << 4)
                        + self.v_addr.yfine() as u16;

                    self.next_tile.lo = self.bus.read_data(vaddr);
                }
                6 => {
                    // Same thing but + 8 for the high bitplane
                    let vaddr = self.ctrl.bgrnd_pattern_addr()
                        + ((self.next_tile.id as u16) << 4)
                        + self.v_addr.yfine() as u16
                        + 8;

                    self.next_tile.hi = self.bus.read_data(vaddr);
                }
                // Increment horizontal scroll
                7 => self.increment_xscroll(),
                _ => {}
            }
        }

        // Increment vertical scrolling.
        if self.cycle == 256 {
            self.increment_yscroll();
        }

        // End of the scanline.
        if self.cycle == 257 {
            // Load the next tile into the shifters.
            self.load_next_tile();

            // Update x coarse and nametable x if background rendering is
            // enabled.
            if self.mask.show_background() {
                self.v_addr.set_nta_h(self.scroll.nta_h());
                self.v_addr.set_xcoarse(self.scroll.xcoarse());
            }
        }
    }

    /// Renders sprites for the current scanline.
    fn render_scanline_sprites(&mut self) {
        if self.cycle == 1 {
            self.clearing_oam = true;
        } else if self.cycle == 64 {
            self.clearing_oam = false;
        }

        // Update foreground shifters
        self.shift_fg();

        // All the sprite evaluation is done in 1 cycle (this is NOT how it is
        // done on the real hardware).
        if self.cycle == 257 && self.scanline >= 0 {
            // Set all the values.
            self.oam2_data[..].fill(Sprite {
                y: 0xFF,
                id: 0xFF,
                attr: 0xFF,
                x: 0xFF,
                index: 0xFF,
            });

            // Reset the shifters.
            self.fg_lo_shift.fill(0);
            self.fg_hi_shift.fill(0);

            let mut sprite_count = 0;
            let sprite_size = if self.ctrl.sprite_size() { 16 } else { 8 };

            // Every sprite attributes in OAM is 4 bytes, thus step by 4
            // 0: Y pos
            // 1: Sprite tile ID
            // 2: Attribute byte
            // 3: X pos
            for index in (0..OAM_SIZE).step_by(4) {
                // Calculate the difference between the scanline and the sprite
                // y value.
                let diff = (self.scanline as u16).wrapping_sub(self.oam_data[index] as u16);

                // Starting from sprite 0, check every sprite if they hit the
                // scanline.
                if (0..sprite_size).contains(&diff) {
                    // If the sprite is visible and there is less than 8 sprite
                    // already visible, add it to secondary OAM.
                    if sprite_count < 8 {
                        self.oam2_data[sprite_count].y = self.oam_data[index];
                        self.oam2_data[sprite_count].id = self.oam_data[index + 1];
                        self.oam2_data[sprite_count].attr = self.oam_data[index + 2];
                        self.oam2_data[sprite_count].x = self.oam_data[index + 3];
                        self.oam2_data[sprite_count].index = index as u8;
                    }

                    // Total number of sprite on the scanline (including
                    // discarded ones).
                    sprite_count += 1;
                }
            }

            // If more than 8 sprites, set the sprite overflow bit.
            self.status.set_sprite_overflow(sprite_count > 8);

            // Visible sprite count.
            self.sprite_count = if sprite_count > 8 { 8 } else { sprite_count };
        }

        if self.cycle == 321 {
            self.load_sprites();
        }
    }

    /// Shifts the background shifters.
    ///
    /// Every 8 cycles, the data for the next tile is loaded into the upper 8
    /// bits of this shift register. Meanwhile, the pixel to render is fetched
    /// from one of the lower 8 bits.
    fn shift_bg(&mut self) {
        if self.mask.show_background() {
            self.bg_lo_shift <<= 1;
            self.bg_hi_shift <<= 1;
            self.bg_attr_lo_shift <<= 1;
            self.bg_attr_hi_shift <<= 1;
        }
    }

    /// Shifts the foreground shifters.
    fn shift_fg(&mut self) {
        if self.mask.show_sprites() && (2..258).contains(&self.cycle) {
            for (i, sprite) in self
                .oam2_data
                .iter_mut()
                .take(self.sprite_count)
                .enumerate()
            {
                if sprite.x > 0 {
                    sprite.x -= 1;
                } else {
                    self.fg_lo_shift[i] <<= 1;
                    self.fg_hi_shift[i] <<= 1;
                }
            }
        }
    }

    /// Loads the next background tile into the shifters.
    fn load_next_tile(&mut self) {
        if self.rendering_enabled() {
            self.bg_lo_shift |= self.next_tile.lo as u16;
            self.bg_hi_shift |= self.next_tile.hi as u16;

            let attr = self.next_tile.attr;
            self.bg_attr_lo_shift |= if attr & 0x1 != 0 { 0xFF } else { 0x00 };
            self.bg_attr_hi_shift |= if attr & 0x2 != 0 { 0xFF } else { 0x00 };
        }
    }

    /// Increment horizontal scroll.
    fn increment_xscroll(&mut self) {
        if self.mask.show_background() {
            let xcoarse = self.v_addr.xcoarse();
            let nta_h = self.v_addr.nta_h();
            if xcoarse == 31 {
                self.v_addr.set_xcoarse(0);
                self.v_addr.set_nta_h(!nta_h);
            } else {
                self.v_addr.set_xcoarse(xcoarse + 1);
            }
        }
    }

    /// Increment vertical scroll.
    fn increment_yscroll(&mut self) {
        if self.mask.show_background() {
            let yfine = self.v_addr.yfine();
            let ycoarse = self.v_addr.ycoarse();
            let nta_v = self.v_addr.nta_v();
            if yfine < 7 {
                self.v_addr.set_yfine(yfine + 1);
            } else {
                self.v_addr.set_yfine(0);
                if ycoarse == 29 {
                    self.v_addr.set_ycoarse(0);
                    self.v_addr.set_nta_v(!nta_v);
                } else if ycoarse == 31 {
                    self.v_addr.set_ycoarse(0);
                } else {
                    self.v_addr.set_ycoarse(ycoarse + 1);
                }
            }
        }
    }

    /// Load sprites from secondary OAM into the shifters.
    fn load_sprites(&mut self) {
        let scanline = self.scanline as u8;

        for i in 0..self.sprite_count {
            let sprite_addr = match !self.ctrl.sprite_size() {
                true => {
                    let offset = self.ctrl.sprite_pattern_addr();
                    let flipped_v = self.oam2_data[i].attr & 0x80 != 0;
                    let tile_id = self.oam2_data[i].id;
                    let row = match flipped_v {
                        true => (7 - (scanline - self.oam2_data[i].y)) as u16,
                        false => (scanline - self.oam2_data[i].y) as u16,
                    };

                    offset | (tile_id as u16) << 4 | row
                }
                false => {
                    let offset = ((self.oam2_data[i].id & 0x01) as u16) << 12;
                    let flipped_v = self.oam2_data[i].attr & 0x80 != 0;
                    let top_half = scanline - self.oam2_data[i].y < 8;
                    let tile_id = match (flipped_v, top_half) {
                        (false, true) | (true, false) => self.oam2_data[i].id & 0xFE,
                        (false, false) | (true, true) => (self.oam2_data[i].id & 0xFE) + 1,
                    };
                    let row = match flipped_v {
                        true => {
                            7_u16.wrapping_sub(scanline.wrapping_sub(self.oam2_data[i].y) as u16)
                                & 0x7
                        }

                        false => ((scanline - self.oam2_data[i].y) & 0x7) as u16,
                    };

                    offset | (tile_id as u16) << 4 | row
                }
            };

            let sprite_lo = self.bus.read_data(sprite_addr);
            let sprite_hi = self.bus.read_data(sprite_addr.wrapping_add(8));

            // Flip horizontal closure.
            let flip_h = |mut v: u8| {
                v = (v & 0xF0) >> 4 | (v & 0x0F) << 4;
                v = (v & 0xCC) >> 2 | (v & 0x33) << 2;
                v = (v & 0xAA) >> 1 | (v & 0x55) << 1;
                v
            };

            self.fg_lo_shift[i] = match self.oam2_data[i].attr & 0x40 != 0 {
                true => flip_h(sprite_lo),
                false => sprite_lo,
            };

            self.fg_hi_shift[i] = match self.oam2_data[i].attr & 0x40 != 0 {
                true => flip_h(sprite_hi),
                false => sprite_hi,
            };
        }
    }
}

impl Ppu for NesPpu<'_> {
    /// Writes value to the address register.
    fn write_addr(&mut self, value: u8) {
        // Because the PPU address is a 14 bit address and the CPU uses an 8 bit
        // bus, we have to write in two steps. The PPU uses a toggle to choose
        // which part of the address to write.
        match self.addr_toggle {
            // If it is set, set the lower bits of the address in the scroll
            // and then set the address register (v register) to the scroll.
            true => {
                self.scroll.set_addr_lo(value);
                self.v_addr = self.scroll;
            }

            // Otherwise, set the high bits of the scroll.
            false => self.scroll.set_addr_hi(value & 0x3F),
        }

        self.addr_toggle = !self.addr_toggle;
    }

    /// Writes to the control register.
    fn write_ctrl(&mut self, value: u8) {
        // Set the register to data
        self.ctrl.update(value);

        // Update scroll nametable
        self.scroll.set_nta_h(self.ctrl.nta_h());
        self.scroll.set_nta_v(self.ctrl.nta_v());
    }

    /// Writes to the mask register.
    fn write_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    /// Writes to the scroll register.
    fn write_scroll(&mut self, value: u8) {
        // Writing to the scroll register uses the same latch as the address
        // register.
        match self.addr_toggle {
            // If it is set, write Y scroll values
            true => {
                self.scroll.set_yfine(value & 0x7);
                self.scroll.set_ycoarse(value >> 3);
            }
            // Otherwise, write X scroll values
            false => {
                self.xfine = value & 0x7;
                self.scroll.set_xcoarse(value >> 3);
            }
        }
        // Update the toggle
        self.addr_toggle = !self.addr_toggle;
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
        let data = self.status.snapshot() | (self.open_bus & 0x1F);
        self.status.reset_vblank_status();
        self.nmi_interrupt = None;
        self.addr_toggle = false;
        data
    }

    fn read_oam_data(&mut self) -> u8 {
        match self.clearing_oam {
            // Always returns 0xFF when clearing secondary OAM
            true => 0xFF,
            false => {
                // Bits 2, 3 and 4 do not exist in the PPU if reading byte 2.
                let mask = match self.oam_addr & 0x3 {
                    2 => 0xE3,
                    _ => 0xFF,
                };

                // Read from OAM and refresh open bus
                self.refresh_open_bus(self.oam_data[self.oam_addr as usize] & mask)
            }
        }
    }

    /// Returns number of frames rendered.
    fn read_frame_count(&self) -> u128 {
        self.frame_count
    }

    fn write_data(&mut self, data: u8) {
        let addr = self.v_addr.raw();
        self.bus.write_data(addr, data);
        self.refresh_open_bus(data);
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        // Reading data takes 2 reads to get the data. The first read put the
        // data in a buffer amd the second read puts the buffer data on the bus.

        // Put the buffer data on the bus.
        let mut result = self.buf;

        // Read new data into the buffer.
        let addr = self.v_addr.raw();
        self.buf = self.bus.read_data(addr);

        // If the data read in from palette RAM, it only takes 1 read
        if (self.v_addr.raw() & 0x3F00) == 0x3F00 {
            // Put the buffer data which was just read on the bus
            result = (self.open_bus & 0xC0) | (self.buf & 0x3F);

            // Add the geryscale mask if enabled.
            result &= self.mask.grayscale_mask();
        }

        self.refresh_open_bus(result);
        self.increment_vram_addr();

        result
    }
}

#[cfg(test)]
pub mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        bus::PPUBus,
        cartridge::{tests::test_cartridge, Mirroring},
    };

    use super::*;

    /// Returns an instatiated PPU with an empty ROM loaded.
    pub fn new_empty_rom_ppu(mirroring: Option<Mirroring>) -> NesPpu<'static> {
        let cart = test_cartridge(vec![], mirroring).unwrap();

        let bus = PPUBus::new(Rc::new(RefCell::new(cart)));
        NesPpu::new(Box::new(bus), |_| {})
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = new_empty_rom_ppu(None);
        ppu.write_addr(0x23);
        ppu.write_addr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.bus.read_data(0x2305), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = new_empty_rom_ppu(None);
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x2305, 0x66);

        ppu.write_addr(0x23);
        ppu.write_addr(0x05);

        ppu.read_data();
        assert_eq!(ppu.v_addr.raw(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = new_empty_rom_ppu(None);
        ppu.write_ctrl(0);
        ppu.bus.write_data(0x21ff, 0x66);
        ppu.bus.write_data(0x2200, 0x77);

        ppu.write_addr(0x21);
        ppu.write_addr(0xff);

        ppu.read_data();
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = new_empty_rom_ppu(None);
        ppu.write_ctrl(0b100);
        ppu.bus.write_data(0x21ff, 0x66);
        ppu.bus.write_data(0x21ff + 32, 0x77);
        ppu.bus.write_data(0x21ff + 64, 0x88);

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
        let mut ppu = new_empty_rom_ppu(None);
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
        let mut ppu = new_empty_rom_ppu(Some(Mirroring::Vertical));

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
        let mut ppu = new_empty_rom_ppu(None);
        ppu.bus.write_data(0x2305, 0x66);

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
    fn test_read_status_resets_vblank() {
        let mut ppu = new_empty_rom_ppu(None);
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = new_empty_rom_ppu(None);
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
        let mut ppu = new_empty_rom_ppu(None);

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
