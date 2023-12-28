use super::LENGTH_TABLE;

/// The sequencer sends the following looping 32-step sequence of values to the
/// mixer.
const OUTPUT_LEVELS: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// Represents the NES triangle channel which generates a pseudo-triangle wave.
/// It has no volume control; the waveform is either cycling or suspended.
pub struct Triangle {
    enabled: bool,
    phase: u8,

    timer_period: u16,
    timer: u16,

    counter_halt: bool,
    length_counter: u8,

    counter_reload: bool,
    counter_period: u8,
    linear_counter: u8,
}

impl Triangle {
    /// Creates a new Triangle register.
    pub fn new() -> Self {
        Self {
            enabled: false,
            phase: 0,

            timer_period: 0,
            timer: 0,

            counter_halt: false,
            length_counter: 0,

            counter_reload: false,
            counter_period: 0,
            linear_counter: 0,
        }
    }

    /// Toggles the channel on or off.
    pub fn toggle(&mut self, enabled: bool) {
        self.enabled = enabled;

        if !self.enabled {
            self.length_counter = 0;
        }
    }

    /// Updates the linear counter.
    ///
    /// Where data is equal to:
    ///
    /// CRRR RRRR
    /// C: Control flag (linear counter halt and length counter halt)
    /// R: Linear counter period
    pub fn write_linear_counter(&mut self, data: u8) {
        self.counter_period = data & 0x7F;
        self.counter_halt = data & 0x80 != 0;
        if self.counter_halt {
            self.linear_counter = self.counter_period;
        }
    }

    /// Sets the timer low.
    ///
    /// Where data is equal to:
    ///
    /// TTTT TTTT
    /// T: Timer period low
    pub fn write_timer_low(&mut self, data: u8) {
        // TTTT TTTT
        // T: Timer period low
        self.timer_period = (self.timer_period & 0xFF00) | data as u16;
    }

    /// Sets the timer high.
    ///
    /// Where data is equal to:
    ///
    /// LLLL LTTT
    /// L: Length counter load
    /// T: Timer period high
    pub fn write_timer_high(&mut self, data: u8) {
        self.timer_period = ((data & 0x7) as u16) << 8 | (self.timer_period & 0xFF);
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize];
        self.counter_reload = true;
    }

    /// Clocks the timer / divider.
    pub fn clock_timer(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        // Reset the timer.
        self.timer = self.timer_period + 1;

        // The sequencer is clocked by the timer, move to the next phase.
        if self.length_counter > 0 && self.linear_counter > 0 && self.timer_period > 1 {
            self.phase = (self.phase + 1) % 32;
        }
    }

    /// Clocks the length counter.
    pub fn clock_length(&mut self) {
        if !self.counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Clocks the linear counter.
    pub fn clock_counter(&mut self) {
        if self.counter_reload {
            self.linear_counter = self.counter_period;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }

        // Stop reloading the counter if the halt flag is set.
        if !self.counter_halt {
            self.counter_reload = false;
        }
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }

    /// Returns the output volume of the channel.
    pub fn output(&self) -> u8 {
        // All the conditions below silence the channel.
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 {
            return 0;
        }

        OUTPUT_LEVELS[self.phase as usize]
    }
}

#[cfg(test)]
mod tests {
    use crate::apu::triangle::OUTPUT_LEVELS;

    use super::Triangle;

    #[test]
    fn test_new() {
        let triangle = Triangle::new();
        assert_eq!(triangle.enabled, false);
        assert_eq!(triangle.phase, 0);
        assert_eq!(triangle.timer_period, 0);
        assert_eq!(triangle.timer, 0);
        assert_eq!(triangle.counter_halt, false);
        assert_eq!(triangle.length_counter, 0);
        assert_eq!(triangle.counter_reload, false);
        assert_eq!(triangle.counter_period, 0);
        assert_eq!(triangle.linear_counter, 0);
    }

    #[test]
    fn test_toggle() {
        let mut triangle = Triangle::new();
        triangle.toggle(true);
        assert_eq!(triangle.enabled, true);
        triangle.toggle(false);
        assert_eq!(triangle.enabled, false);
        assert_eq!(triangle.length_counter, 0);
    }

    #[test]
    fn test_write_linear_counter() {
        let mut triangle = Triangle::new();
        triangle.write_linear_counter(0x8F);
        assert_eq!(triangle.counter_period, 0x0F);
        assert_eq!(triangle.counter_halt, true);
        assert_eq!(triangle.linear_counter, 0x0F);
    }

    #[test]
    fn test_clock_length() {
        let mut triangle = Triangle::new();
        triangle.length_counter = 5;
        triangle.clock_length();
        assert_eq!(triangle.length_counter, 4);
    }

    #[test]
    fn test_clock_counter() {
        let mut triangle = Triangle::new();
        triangle.counter_reload = true;
        triangle.counter_period = 5;
        triangle.clock_counter();
        assert_eq!(triangle.linear_counter, 5);
    }

    #[test]
    fn test_length_counter() {
        let triangle = Triangle::new();
        assert_eq!(triangle.length_counter(), 0);
    }

    #[test]
    fn test_output() {
        let mut triangle = Triangle::new();
        assert_eq!(triangle.output(), 0);
        triangle.enabled = true;
        triangle.length_counter = 5;
        triangle.linear_counter = 5;
        assert_eq!(triangle.output(), OUTPUT_LEVELS[triangle.phase as usize]);
    }
}
