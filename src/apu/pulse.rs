use crate::apu::LENGTH_TABLE;

// 0 - 0 1 0 0 0 0 0 0 (12.5%)
// 1 - 0 1 1 0 0 0 0 0 (25%)
// 2 - 0 1 1 1 1 0 0 0 (50%)
// 3 - 1 0 0 1 1 1 1 1 (25% negated)
/// Table of the different duty cycles
const DUTY_TABLE: [u8; 4] = [0b0100_0000, 0b0110_0000, 0b0111_1000, 0b1001_1111];

/// Channel 1 or 2
pub enum Channel {
    One,
    Two,
}

/// Represents the NES pulse (square) channel which generate a pulse wave with
/// variable duty.
pub struct Pulse {
    enabled: bool,

    // A duty cycle describes the fraction of one period in which a signal or
    // system is active.
    duty_cycle: u8,

    // Current phase of the duty cycle. It's used to determine the current point
    // in the duty cycle pattern.
    duty_phase: u8,

    constant_volume: bool,
    volume: u8,

    length_halt: bool,
    length_counter: u8,

    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_timer: u8,

    timer: u16,
    timer_period: u16,

    envelope_loop: bool,
    envelope_period: u8,
    envelope_timer: u8,
    envelope_volume: u8,
}

impl Pulse {
    /// Creates a new Pulse struct.
    pub fn new() -> Self {
        Self {
            enabled: false,

            duty_cycle: 0,
            duty_phase: 0,
            constant_volume: false,
            volume: 0,

            length_halt: false,
            length_counter: 0,

            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_timer: 0,

            timer: 0,
            timer_period: 0,

            envelope_loop: false,
            envelope_period: 0,
            envelope_timer: 0,
            envelope_volume: 0,
        }
    }

    /// Resets the Pulse struct.
    pub fn reset(&mut self) {
        self.enabled = false;

        self.duty_cycle = 0;
        self.duty_phase = 0;
        self.constant_volume = false;
        self.volume = 0;

        self.length_halt = false;
        self.length_counter = 0;

        self.sweep_enabled = false;
        self.sweep_period = 0;
        self.sweep_negate = false;
        self.sweep_shift = 0;
        self.sweep_timer = 0;

        self.timer = 0;
        self.timer_period = 0;

        self.envelope_loop = false;
        self.envelope_period = 0;
        self.envelope_timer = 0;
        self.envelope_volume = 0;
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
    /// DDLC VVVV
    /// D: Duty cycle
    /// L: Envelope loop / length counter halt
    /// C: Output constant volume
    /// V: Volume value / envelope period
    pub fn write_volume(&mut self, data: u8) {
        self.duty_cycle = data >> 0x6;
        self.length_halt = (data & 0x20) != 0;
        self.envelope_loop = self.length_halt;
        self.constant_volume = (data & 0x10) != 0;
        self.volume = data & 0xF;
        self.envelope_period = self.volume;
    }

    /// Sets the sweep unit used to manipulate the frequency of the pulse.
    ///
    /// Where data is equal to:
    ///
    /// EPPP NSSS
    /// E: Enabled
    /// P: Period
    /// N: Negate
    /// S: Shift
    pub fn write_sweep(&mut self, data: u8) {
        self.sweep_enabled = (data & 0x80) != 0;
        self.sweep_period = (data >> 0x4) & 7;
        self.sweep_negate = (data & 0x8) != 0;
        self.sweep_shift = data & 0x7;

        // A write to this register reloads the sweep
        self.sweep_timer = self.sweep_period + 1;
    }

    /// Sets the timer low.
    ///
    /// Where data is equal to:
    ///
    /// TTTT TTTT
    /// T: Timer period low
    pub fn write_timer_low(&mut self, data: u8) {
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
        self.timer_period = (self.timer_period & 0x00FF) | ((data as u16 & 0x7) << 8);
        self.length_counter = LENGTH_TABLE[(data >> 3) as usize];

        // A write to this register reloads the length counter, restarts the
        // envelope, and resets the phase of the pulse generator.
        //
        // See: https://www.nesdev.org/wiki/APU#Pulse_($4000%E2%80%93$4007)
        self.duty_phase = 0;
        self.envelope_volume = 15;
        self.envelope_timer = self.envelope_period + 1;
    }

    /// Clocks the timer / divider.
    pub fn clock_timer(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        self.timer = self.timer_period + 1;
        self.duty_phase = (self.duty_phase + 1) % 8;
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
        match self.envelope_timer > 0 {
            true => self.envelope_timer -= 1,
            false => {
                self.envelope_timer = self.envelope_period + 1;

                if self.envelope_volume > 0 && !self.envelope_loop {
                    self.envelope_volume -= 1;
                } else if self.envelope_volume < 15 && self.envelope_loop {
                    self.envelope_volume += 1;
                }
            }
        }
    }

    /// Clock the sweep unit which periodically adjusts the timer period.
    pub fn clock_sweep(&mut self, chan: Channel) {
        match self.sweep_timer > 0 {
            true => self.sweep_timer -= 1,
            false => {
                if self.sweep_enabled && self.timer_period > 7 && self.sweep_shift > 0 {
                    self.sweep(chan);
                }

                self.sweep_timer = self.sweep_period + 1;
            }
        }
    }

    /// Returns the output volume of the channel
    pub fn output(&self) -> u8 {
        let dt = DUTY_TABLE[self.duty_cycle as usize];
        let dp = 1 << self.duty_phase;
        let duty = (dt & dp) != 0;

        if !self.enabled
            || self.timer_period > 0x7FF
            || self.length_counter == 0
            || self.timer_period < 8
            || !duty
        {
            return 0;
        }

        match self.constant_volume {
            true => self.volume,
            false => self.envelope_volume,
        }
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }

    /// Adjusts the timer period based on the given channel.
    fn sweep(&mut self, chan: Channel) {
        let delta = self.timer_period >> self.sweep_shift;

        self.timer_period = match self.sweep_negate {
            true => match chan {
                Channel::One => self.timer_period.wrapping_add(!delta),
                Channel::Two => self.timer_period.wrapping_sub(delta),
            },
            false => self.timer_period.wrapping_add(delta),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output() {
        let mut pulse = Pulse::new();
        pulse.duty_cycle = 3;
        pulse.duty_phase = 1;
        pulse.length_halt = true;
        pulse.timer_period = 0x7F0;
        pulse.length_counter = 10;
        pulse.constant_volume = true;
        pulse.volume = 5;
        assert_eq!(pulse.output(), 5);
    }

    #[test]
    fn test_length_counter() {
        let mut pulse = Pulse::new();
        pulse.length_counter = 10;
        assert_eq!(pulse.length_counter(), 10);
    }

    #[test]
    fn test_sweep() {
        let mut pulse = Pulse::new();
        pulse.timer_period = 100;
        pulse.sweep_shift = 2;
        pulse.sweep_negate = true;
        pulse.sweep(Channel::One);
        assert_eq!(pulse.timer_period, 74);
    }
}
