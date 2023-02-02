const NOTUSED: u8 = 0b00000001;
const NOTUSED2: u8 = 0b00000010;
const NOTUSED3: u8 = 0b00000100;
const NOTUSED4: u8 = 0b00001000;
const NOTUSED5: u8 = 0b00010000;
const SPRITE_OVERFLOW: u8 = 0b00100000;
const SPRITE_ZERO_HIT: u8 = 0b01000000;
const VBLANK_STARTED: u8 = 0b10000000;

/// Represents the PPU status register.
pub struct Status {
    /// 7  bit  0
    /// ---- ----
    /// V S O . . . . .
    /// | | | | | | | |
    /// | | | + - + + + +- Least significant bits previously written into a PPU register
    /// | | |              (due to register not being updated for this address)
    /// | | +------------- Sprite overflow. The intent was for this flag to be set
    /// | |                whenever more than eight sprites appear on a scanline, but a
    /// | |                hardware bug causes the actual behavior to be more complicated
    /// | |                and generate false positives as well as false negatives; see
    /// | |                PPU sprite evaluation. This flag is set during sprite
    /// | |                evaluation and cleared at dot 1 (the second dot) of the
    /// | |                pre-render line.
    /// | +--------------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
    /// |                  a nonzero background pixel; cleared at dot 1 of the pre-render
    /// |                  line. Used for raster timing.
    /// +----------------- Vertical blank has started (0: not in vblank; 1: in vblank).
    ///                    Set at dot 1 of line 241 (the line *after* the post-render
    ///                    line); cleared after reading $2002 and at dot 1 of the
    ///                    pre-render line.
    bits: u8,
}

impl Status {
    /// Returns a new status register.
    pub fn new() -> Self {
        Status { bits: 0b00000000 }
    }

    /// Sets the VBLANK status.
    pub fn set_vblank_status(&mut self, status: bool) {
        if status {
            self.bits |= VBLANK_STARTED
        } else {
            self.bits &= !VBLANK_STARTED
        }
    }

    /// Sets sprite zero hit status.
    pub fn set_sprite_zero_hit(&mut self, status: bool) {
        if status {
            self.bits |= SPRITE_ZERO_HIT
        } else {
            self.bits &= !SPRITE_ZERO_HIT
        }
    }

    /// Sets sprite zero overflow.
    pub fn set_sprite_overflow(&mut self, status: bool) {
        if status {
            self.bits |= SPRITE_OVERFLOW
        } else {
            self.bits &= !SPRITE_OVERFLOW
        }
    }

    /// Resets VBLANK status.
    pub fn reset_vblank_status(&mut self) {
        self.bits &= !VBLANK_STARTED
    }

    /// Returns true if in VBLANK.
    pub fn is_in_vblank(&self) -> bool {
        self.bits & VBLANK_STARTED == VBLANK_STARTED
    }

    /// Returns current status of the register.
    pub fn snapshot(&self) -> u8 {
        self.bits
    }
}
