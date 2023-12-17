/// Represents the NES Noise channel which generates pseudo-random 1-bit noise
/// at 16 different frequencies.
pub struct Noise {
    length_counter: u8,
}

impl Noise {
    /// Creates a new Noise register.
    pub fn new() -> Self {
        Self { length_counter: 0 }
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }
}
