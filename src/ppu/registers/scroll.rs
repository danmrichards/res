// Represents the PPU scroll register.
pub struct Scroll {
    pub x: u8,
    pub y: u8,
    pub latch: bool,
}

impl Scroll {
    pub fn new() -> Self {
        Scroll {
            x: 0,
            y: 0,
            latch: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.latch {
            self.x = data;
        } else {
            self.y = data;
        }
        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
