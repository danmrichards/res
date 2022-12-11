// Represents the PPU address register.
pub struct Addr {
    hi: u8,
    lo: u8,
    write_hi: bool,
}

impl Addr {
    // Returns an instantiated address register.
    pub fn new() -> Self {
        Addr {
            hi: 0,
            lo: 0,
            write_hi: true,
        }
    }

    // Sets two bits of the register with data.
    fn set(&mut self, data: u16) {
        self.hi = (data >> 8) as u8;
        self.lo = (data & 0xFF) as u8;
    }

    // Sets either the high or low bit of the register depending on the "high
    // pointer".
    pub fn update(&mut self, data: u8) {
        if self.write_hi {
            self.hi = data;
        } else {
            self.lo = data;
        }

        // Mirror down addr above 0x3FFF.
        if self.get() > 0x3FFF {
            self.set(self.get() & 0b11111111111111);
        }
        self.write_hi = !self.write_hi;
    }

    // Increments the register.
    pub fn increment(&mut self, inc: u8) {
        let lo = self.lo;
        self.lo = self.lo.wrapping_add(inc);
        if lo > self.lo {
            self.hi = self.hi.wrapping_add(1);
        }

        // Mirror down addr above 0x3FFF.
        if self.get() > 0x3FFF {
            self.set(self.get() & 0b11111111111111);
        }
    }

    // Resets the register.
    pub fn reset(&mut self) {
        self.write_hi = true;
    }

    // Returns the value of the register.
    pub fn get(&self) -> u16 {
        ((self.hi as u16) << 8) | (self.lo as u16)
    }
}
