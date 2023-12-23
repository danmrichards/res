const RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

/// Represents the NES delta modulation channel (DMC) which can output 1-bit
/// delta-encoded samples or can have its 7-bit counter directly loaded,
/// allowing flexible manual sample playback.
pub struct Dmc {
    enabled: bool,

    disable_interrupt: bool,
    pending_interrupt: Option<bool>,

    loop_sample: bool,
    rate: u16,
    rate_counter: u16,

    pending_read: Option<bool>,
    addr: u8,
    last_addr: u16,
    buf: u8,
    phase: u8,

    output_level: u8,
    length_counter: u16,
    pcm_length: u16,
}

impl Dmc {
    /// Creates a new DMC.
    pub fn new() -> Self {
        Self {
            enabled: false,
            disable_interrupt: false,
            pending_interrupt: None,
            loop_sample: false,
            rate: 0,
            rate_counter: 0,
            pending_read: None,
            addr: 0,
            last_addr: 0xC000,
            buf: 0,
            phase: 0,
            output_level: 0,
            length_counter: 0,
            pcm_length: 0,
        }
    }

    /// Toggles the channel on or off.
    pub fn toggle(&mut self, enabled: bool) {
        self.enabled = enabled;

        if !self.enabled {
            self.length_counter = 0;
        } else if self.length_counter == 0 {
            self.length_counter = self.pcm_length * 16 + 1;
        }
    }

    /// Writes the sample frequency.
    ///
    /// Where data is equal to:
    ///
    /// IL-- RRRR
    /// I: IRQ enable
    /// L: Loop flag
    /// R: Rate index (frequency)
    pub fn write_sample_frequency(&mut self, data: u8) {
        self.rate = RATE_TABLE[(data & 0xF) as usize];
        self.loop_sample = data & 0x40 != 0;
        self.disable_interrupt = data & 0x80 != 0;
    }

    /// Writes a raw PCM sample.
    ///
    /// Where data is equal to:
    ///
    /// -DDD DDDD
    /// D: Raw PCM sample
    pub fn write_raw_sample(&mut self, data: u8) {
        self.output_level = data & 0x7F;
    }

    /// Writes the start address of the sample.
    ///
    /// Where data is equal to:
    ///
    /// AAAA AAAA
    /// A: Start address
    pub fn write_sample_start(&mut self, data: u8) {
        self.addr = data;
        self.last_addr = 0xC000 + (data as u16 * 64);
    }

    /// Writes the length of the sample.
    ///
    /// Where data is equal to:
    ///
    /// LLLL LLLL
    /// L: Sample length (how many samples to play)
    pub fn write_sample_length(&mut self, data: u8) {
        self.pcm_length = data as u16;
        self.length_counter = self.pcm_length * 16 + 1;
    }

    /// Clocks the DMC.
    pub fn clock(&mut self) {
        if self.rate_counter > 0 {
            self.rate_counter -= 1;
            return;
        }

        self.clock_timer();
        self.rate_counter = self.rate;
    }

    /// Clocks the DMC timer.
    fn clock_timer(&mut self) {
        // Phase 0 means the PCM or DPCM sample has been played
        if self.phase == 0 {
            // If the length counter == 0 (all the samples have been played)
            // and the loop flag is set, we load the start address and
            // reset the length counter
            if self.length_counter == 0 && self.loop_sample {
                self.length_counter = self.pcm_length * 16 + 1;
                self.last_addr = 0xC000 + (self.addr as u16 * 64);
            }

            if self.length_counter > 0 {
                self.pending_read = Some(true);
                self.phase = 8;
                self.length_counter -= 1;
            } else {
                if !self.disable_interrupt {
                    self.pending_interrupt = Some(true);
                }

                self.enabled = false;
            }
        }

        // Sample is still playing.
        if self.phase != 0 {
            self.phase -= 1;

            // Adjust the volume.
            let delta = (self.buf & (0x80 >> self.phase)) != 0;
            let vol = match delta {
                true => self.output_level.wrapping_add(2),
                false => self.output_level.wrapping_sub(2),
            };

            if (0..=0x7F).contains(&vol) && self.enabled {
                self.output_level = vol;
            }
        }
    }

    /// Returns the address of the next sample
    pub fn address(&self) -> u16 {
        self.last_addr
    }

    /// Sets the audio sample of the channel
    pub fn set_sample(&mut self, sample: u8) {
        self.buf = sample;

        self.last_addr = self.last_addr.wrapping_add(1) | 0x8000;
    }

    /// Returns if the channel needs a sample or not
    pub fn need_sample(&mut self) -> bool {
        self.pending_read.take().is_some()
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u16 {
        self.length_counter
    }

    /// Returns true if the DMC chanel is waiting for an interrupt.
    pub fn poll_interrupt(&mut self) -> bool {
        self.pending_interrupt.take().is_some()
    }

    /// Returns the output volume of the channel
    pub fn output(&self) -> u8 {
        0
    }
}
