use super::LENGTH_TABLE;

const TIMER_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// Represents the NES Noise channel which generates pseudo-random 1-bit noise
/// at 16 different frequencies.
pub struct Noise {
    enabled: bool,
    mode: bool,

    timer: u16,
    timer_period: u16,

    length_halt: bool,
    length_counter: u8,

    constant_volume: bool,
    volume: u8,

    envelope_timer: u8,
    envelope_volume: u8,

    shift: u16,
}

impl Noise {
    /// Creates a new Noise register.
    pub fn new() -> Self {
        Self {
            enabled: false,
            mode: false,
            length_counter: 0,
            timer: 0,
            timer_period: 0,
            length_halt: false,
            constant_volume: false,
            volume: 0,
            envelope_timer: 0,
            envelope_volume: 0,
            shift: 0,
        }
    }

    /// Toggles the channel on or off.
    pub fn toggle(&mut self, enabled: bool) {
        self.enabled = enabled;

        if !self.enabled {
            self.length_counter = 0;
        }
    }

    /// Sets the width of the pulse.
    ///
    /// Where data is equal to:
    ///
    /// --LC VVVV
    /// L: Envelope loop / length counter halt
    /// C: Output constant volume
    /// V: Volume value / envelope period
    pub fn write_volume(&mut self, data: u8) {
        self.length_halt = data & 0x20 != 0;
        self.constant_volume = data & 0x10 != 0;
        self.volume = data & 0xF;
    }

    /// Sets the timer low.
    ///
    /// Where data is equal to:
    ///
    /// M--- PPPP
    /// M: Mode flag
    /// P: Timer period table index
    pub fn write_timer_low(&mut self, data: u8) {
        self.mode = data & 0x80 != 0;
        self.timer_period = TIMER_PERIODS[(data & 0xF) as usize];
    }

    /// Sets the timer high.
    ///
    /// Where data is equal to:
    ///
    /// LLLL L---
    /// L: Length counter table index
    pub fn write_timer_high(&mut self, data: u8) {
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize];
        self.envelope_volume = 15;
        self.envelope_timer = self.volume + 1;
    }

    /// Clocks the timer / divider.
    pub fn clock_timer(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        self.timer = self.timer_period;

        let bit = if self.mode { 6 } else { 1 };

        let feedback = (self.shift ^ (self.shift >> bit)) & 0x1;
        self.shift = (self.shift >> 1) | (feedback << 14);
    }

    /// Clocks the length counter.
    pub fn clock_length(&mut self) {
        if self.length_counter > 0 && !self.length_halt {
            self.length_counter -= 1;
        }
    }

    /// Clocks the envelope.
    ///
    /// Depending on the nature of the envelope, it will either increment or
    /// decrement the volume. This can be used to create a constant volume or
    /// a increasing/decreasing volume.
    pub fn clock_envelope(&mut self) {
        if self.envelope_timer > 0 {
            self.envelope_timer -= 1;
            return;
        }

        if self.envelope_volume > 0 {
            self.envelope_volume -= 1;
        } else if self.length_halt {
            self.envelope_volume = 15;
        }

        self.envelope_timer = self.volume + 1;
    }

    /// Returns the length counter value.
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }

    /// Returns the output volume of the channel.
    pub fn output(&self) -> u8 {
        // All the conditions below silence the channel.
        if !self.enabled || self.length_counter == 0 || self.shift & 0x1 != 0 {
            return 0;
        }

        // Check if we should output constant volume or the envelope volume
        match self.constant_volume {
            true => self.volume,
            false => self.envelope_volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::apu::{noise::TIMER_PERIODS, LENGTH_TABLE};

    use super::Noise;

    #[test]
    fn test_new() {
        let noise = Noise::new();
        assert!(!noise.enabled);
        assert!(!noise.mode);
        assert_eq!(noise.length_counter, 0);
        assert_eq!(noise.timer, 0);
        assert_eq!(noise.timer_period, 0);
        assert!(!noise.length_halt);
        assert!(!noise.constant_volume);
        assert_eq!(noise.volume, 0);
        assert_eq!(noise.envelope_timer, 0);
        assert_eq!(noise.envelope_volume, 0);
        assert_eq!(noise.shift, 0);
    }

    #[test]
    fn test_toggle() {
        let mut noise = Noise::new();
        noise.toggle(true);
        assert!(noise.enabled);
        noise.toggle(false);
        assert!(!noise.enabled);
        assert_eq!(noise.length_counter, 0);
    }

    #[test]
    fn test_write_volume() {
        let mut noise = Noise::new();
        noise.write_volume(0x3F);
        assert!(noise.length_halt);
        assert!(noise.constant_volume);
        assert_eq!(noise.volume, 0xF);
    }

    #[test]
    fn test_write_timer_low() {
        let mut noise = Noise::new();
        noise.write_timer_low(0x8F);
        assert!(noise.mode);
        assert_eq!(noise.timer_period, TIMER_PERIODS[0xF]);
    }

    #[test]
    fn test_write_timer_high() {
        let mut noise = Noise::new();
        noise.write_timer_high(0xF8);
        assert_eq!(noise.length_counter, LENGTH_TABLE[0x1F]);
        assert_eq!(noise.envelope_volume, 15);
        assert_eq!(noise.envelope_timer, noise.volume + 1);
    }

    #[test]
    fn test_clock_timer() {
        let mut noise = Noise::new();
        noise.timer = 5;
        noise.clock_timer();
        assert_eq!(noise.timer, 4);
    }

    #[test]
    fn test_clock_length() {
        let mut noise = Noise::new();
        noise.length_counter = 5;
        noise.clock_length();
        assert_eq!(noise.length_counter, 4);
    }

    #[test]
    fn test_clock_envelope() {
        let mut noise = Noise::new();
        noise.envelope_timer = 5;
        noise.clock_envelope();
        assert_eq!(noise.envelope_timer, 4);
    }

    #[test]
    fn test_length_counter() {
        let noise = Noise::new();
        assert_eq!(noise.length_counter(), 0);
    }

    #[test]
    fn test_output() {
        let mut noise = Noise::new();
        assert_eq!(noise.output(), 0);
        noise.enabled = true;
        noise.length_counter = 5;
        noise.shift = 0;
        assert_eq!(noise.output(), noise.envelope_volume);
    }
}
