/// Represents the NES triangle channel which generates a pseudo-triangle wave.
/// It has no volume control; the waveform is either cycling or suspended.
pub struct Triangle {
    length_counter: u8,
}

impl Triangle {
    /// Creates a new Triangle register.
    pub fn new() -> Self {
        Self { length_counter: 0 }
    }

    /// Resets the Triangle register.
    pub fn reset(&mut self) {
        self.length_counter = 0;
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }
}
