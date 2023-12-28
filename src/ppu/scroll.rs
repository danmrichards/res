const XCOARSE_MASK: u16 = 0b11111;
const YCOARSE_MASK: u16 = 0b11111;
const NTA_H_MASK: u16 = 0b1;
const NTA_V_MASK: u16 = 0b1;
const YFINE_MASK: u16 = 0b111;

const XCOARSE_SHIFT: u16 = 0;
const YCOARSE_SHIFT: u16 = 5;
const NTA_H_SHIFT: u16 = 10;
const NTA_V_SHIFT: u16 = 11;
const YFINE_SHIFT: u16 = 12;

/// Represents the PPU scroll register.
#[derive(Default, Clone, Copy)]
pub struct Scroll {
    xcoarse: u8,
    ycoarse: u8,
    nta_h: bool,
    nta_v: bool,
    yfine: u8,
}

impl Scroll {
    /// Returns a new scroll register.
    pub fn new() -> Self {
        Scroll {
            xcoarse: 0,
            ycoarse: 0,
            nta_h: false,
            nta_v: false,
            yfine: 0,
        }
    }

    /// X coarse value
    pub fn xcoarse(&self) -> u8 {
        self.xcoarse
    }

    /// Set x coarse value
    pub fn set_xcoarse(&mut self, v: u8) {
        self.xcoarse = v & XCOARSE_MASK as u8;
    }

    /// Y coarse value
    pub fn ycoarse(&self) -> u8 {
        self.ycoarse
    }

    /// Set y coarse value
    pub fn set_ycoarse(&mut self, v: u8) {
        self.ycoarse = v & YCOARSE_MASK as u8;
    }

    /// Y fine value
    pub fn yfine(&self) -> u8 {
        self.yfine
    }

    /// Set y fine value
    pub fn set_yfine(&mut self, v: u8) {
        self.yfine = v & YFINE_MASK as u8;
    }

    /// Nametable H value
    pub fn nta_h(&self) -> bool {
        self.nta_h
    }

    /// Set nametable H value
    pub fn set_nta_h(&mut self, v: bool) {
        self.nta_h = v;
    }

    /// Nametable V value
    pub fn nta_v(&self) -> bool {
        self.nta_v
    }

    /// Set nametable V value
    pub fn set_nta_v(&mut self, v: bool) {
        self.nta_v = v;
    }

    /// Nametable address
    pub fn nta_addr(&self) -> u16 {
        ((self.nta_v as u16) << NTA_V_SHIFT) | ((self.nta_h as u16) << NTA_H_SHIFT)
    }

    /// Set address low bits
    pub fn set_addr_lo(&mut self, v: u8) {
        self.xcoarse = v & 0b0001_1111;
        self.ycoarse &= 0b0001_1000;
        self.ycoarse |= v >> 5;
    }

    /// Set address high bits
    pub fn set_addr_hi(&mut self, v: u8) {
        self.ycoarse &= 0b0000_0111;
        self.ycoarse |= (v & 0b0000_0011) << 3;
        self.nta_h = v & 0b0000_0100 != 0;
        self.nta_v = v & 0b0000_1000 != 0;
        self.yfine = v >> 4;
        self.yfine &= 0b0000_0111;
    }

    /// Returns the raw address value
    ///
    /// -yyy VHYY YYYX XXXX
    ///
    /// X: X coarse
    ///
    /// Y: Y coarse
    ///
    /// H: Nametable H
    ///
    /// V: Nametable V
    ///
    /// y: Y fine
    pub fn raw(&self) -> u16 {
        (self.xcoarse as u16) << XCOARSE_SHIFT
            | (self.ycoarse as u16) << YCOARSE_SHIFT
            | (self.nta_h as u16) << NTA_H_SHIFT
            | (self.nta_v as u16) << NTA_V_SHIFT
            | (self.yfine as u16) << YFINE_SHIFT
    }

    /// Set all the register bits at once
    pub fn set_raw(&mut self, v: u16) {
        self.xcoarse = ((v & (XCOARSE_MASK << XCOARSE_SHIFT)) >> XCOARSE_SHIFT) as u8;
        self.ycoarse = ((v & (YCOARSE_MASK << YCOARSE_SHIFT)) >> YCOARSE_SHIFT) as u8;
        self.nta_h = (v & (NTA_H_MASK << NTA_H_SHIFT)) != 0;
        self.nta_v = (v & (NTA_V_MASK << NTA_V_SHIFT)) != 0;
        self.yfine = ((v & (YFINE_MASK << YFINE_SHIFT)) >> YFINE_SHIFT) as u8;
    }

    /// Address of the next tile
    ///
    // 0x2000 to offset in VRAM space
    // The lower 12 bits of the address register represent an index
    // in one of the four nametables
    //
    /// 0010 VHYY YYYX XXXX
    ///
    /// V: Nametable V
    ///
    /// H: Nametable H
    ///
    /// Y: Coarse Y
    ///
    /// X: Coarse X
    //
    //   0                1
    // 0 +----------------+----------------+
    //   |                |                |
    //   |                |                |
    //   |    (32x32)     |    (32x32)     |
    //   |                |                |
    //   |                |                |
    // 1 +----------------+----------------+
    //   |                |                |
    //   |                |                |
    //   |    (32x32)     |    (32x32)     |
    //   |                |                |
    //   |                |                |
    //   +----------------+----------------+
    pub fn tile_addr(&self) -> u16 {
        0x2000 | (self.raw() & 0xFFF)
    }

    /// Address of the next tile attribute byte
    ///
    /// 0010 0011 11YY YXXX
    ///
    /// Y: Higher 3 bits of Y coarse
    ///
    /// X: Higher 3 bits of X coarse
    //
    // The last 2 row (last 64 bytes) of each nametable columns are attribute bytes
    pub fn tile_attr_addr(&self) -> u16 {
        0x23C0
            | self.nta_addr()
            | ((self.ycoarse() & 0x1C) << 1) as u16
            | (self.xcoarse() >> 2) as u16
    }
}
