pub mod registers;

/// Length counter values table
/// http://wiki.nesdev.com/w/index.php/APU_Length_Counter
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

/// Pulse 1 registers.
const PULSE1_VOLUME: u16 = 0x4000;
const PULSE1_SWEEP: u16 = 0x4001;
const PULSE1_TIMER_LOW: u16 = 0x4002;
const PULSE1_TIMER_HIGH: u16 = 0x4003;

/// Pulse 2 registers.
const PULSE2_VOLUME: u16 = 0x4004;
const PULSE2_SWEEP: u16 = 0x4005;
const PULSE2_TIMER_LOW: u16 = 0x4006;
const PULSE2_TIMER_HIGH: u16 = 0x4007;

/// Sound status / enable register
const STATUS_REGISTER: u16 = 0x4015;

use registers::pulse::Pulse;

/// Represents the NES Audio Processing Unit (APU).
pub struct Apu {
    pulse1: Pulse,
    pulse2: Pulse,
}

impl Apu {
    /// Creates a new APU.
    pub fn new() -> Self {
        Self {
            pulse1: Pulse::new(),
            pulse2: Pulse::new(),
        }
    }

    /// Resets the APU.
    pub fn reset(&mut self) {
        self.pulse1.reset();
        self.pulse2.reset();
    }

    /// Advances the state of the APU by one CPU cycle.
    pub fn clock(&mut self) {
        panic!("Not implemented")
    }

    /// Reads a byte from the APU.
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            STATUS => self.status(),
            _ => 0,
        }
    }

    /// Writes a byte to the APU.
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            PULSE1_VOLUME => self.pulse1.write_volume(data),
            PULSE1_SWEEP => self.pulse1.write_sweep(data),
            PULSE1_TIMER_LOW => self.pulse1.write_timer_low(data),
            PULSE1_TIMER_HIGH => self.pulse1.write_timer_high(data),

            PULSE2_VOLUME => self.pulse2.write_volume(data),
            PULSE2_SWEEP => self.pulse2.write_sweep(data),
            PULSE2_TIMER_LOW => self.pulse2.write_timer_low(data),
            PULSE2_TIMER_HIGH => self.pulse2.write_timer_high(data),
            _ => (),
        }
    }

    /// Returns an audio sample from the APU.
    pub fn output(&self) -> f32 {
        panic!("Not implemented")
    }

    /// Returns the status of the APU.
    fn status(&self) -> u8 {
        let mut status = 0;
        if self.pulse1.length_counter() > 0 {
            status |= 1;
        }
        if self.pulse2.length_counter() > 0 {
            status |= 2;
        }
        // Repeat for the other channels...
        status
    }
}
