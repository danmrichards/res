mod dmc;
mod noise;
mod pulse;
mod triangle;

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

/// Frame counter register
const FRAME_COUNTER: u16 = 0x4017;

use crate::filters::{Filter, HighPass, LowPass};
use dmc::Dmc;
use noise::Noise;
use pulse::Pulse;
use triangle::Triangle;

/// The mode in which the APU which loop over events.
#[derive(PartialEq)]
enum SequencerMode {
    FourStep,
    FiveStep,
}

/// Represents the NES Audio Processing Unit (APU).
pub struct Apu {
    cycles: u32,
    frame_counter: u16,
    disable_interrupt: bool,
    pending_interrupt: Option<bool>,

    sequencer: u8,
    mode: SequencerMode,

    pulse1: Pulse,
    pulse2: Pulse,
    tri: Triangle,
    noise: Noise,
    dmc: Dmc,

    filters: Vec<Box<dyn Filter>>,
}

impl Apu {
    /// Creates a new APU.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            cycles: 0,
            frame_counter: 0,
            disable_interrupt: false,
            pending_interrupt: None,

            sequencer: 0,
            mode: SequencerMode::FourStep,

            pulse1: Pulse::new(),
            pulse2: Pulse::new(),
            tri: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),

            // The NES hardware follows the DACs with a surprisingly involved
            // circuit that adds several low-pass and high-pass filters:
            //
            // A first-order high-pass filter at 90 Hz
            // Another first-order high-pass filter at 440 Hz
            // A first-order low-pass filter at 14 kHz
            filters: vec![
                Box::new(HighPass::new(sample_rate, 90.0)),
                Box::new(HighPass::new(sample_rate, 440.0)),
                Box::new(LowPass::new(sample_rate, 14000.0)),
            ],
        }
    }

    /// Resets the APU.
    pub fn reset(&mut self) {
        self.cycles = 0;
        self.frame_counter = 0;
        self.sequencer = 0;
        self.disable_interrupt = false;
        self.pending_interrupt = None;

        self.pulse1.reset();
        self.pulse2.reset();
        self.tri.reset();
        self.noise.reset();
        self.dmc.reset();
    }

    /// Advances the state of the APU by one CPU cycle.
    pub fn clock(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);

        // TODO: Update triangle channel timer and tick DMC channel.

        // Pulse and noise channels are clocked at half the rate of the CPU.
        if self.cycles % 2 == 0 {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            // self.noise.clock();
        }

        // TODO: Don't understand any of this frame counter stuff!
        self.frame_counter = self.frame_counter.wrapping_add(2);
        if self.frame_counter >= 14915 {
            self.frame_counter -= 14915;

            self.sequencer = self.sequencer.wrapping_add(1);
            match self.mode {
                SequencerMode::FourStep => self.sequencer %= 4,
                SequencerMode::FiveStep => self.sequencer %= 5,
            }

            // Four step mode can request an interrupt on the last step
            if !self.disable_interrupt
                && self.mode == SequencerMode::FourStep
                && self.sequencer == 0
            {
                self.pending_interrupt = Some(true);
            }

            let half_tick = (self.frame_counter & 0x5) == 1;
            let full_tick = self.sequencer < 4;

            // Sweep tick and length tick
            if half_tick {
                self.pulse1.clock_length();
                self.pulse2.clock_length();
                self.pulse1.clock_sweep(pulse::Channel::One);
                self.pulse2.clock_sweep(pulse::Channel::Two);

                // TODO: Noise and triangle.
            }

            if full_tick {
                self.pulse1.clock_envelope();
                self.pulse2.clock_envelope();

                // TODO: Noise and triangle.
            }
        }
    }

    /// Reads a byte from the APU.
    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            STATUS_REGISTER => self.status(),
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

            // ---D NT21
            // Enable DMC (D), noise (N), triangle (T), and pulse channels (2/1)
            STATUS_REGISTER => {
                self.pulse1.toggle(data & 0x1 != 0);
                self.pulse2.toggle(data & 0x2 != 0);

                // TODO: Triangle, noise, and DMC.
            }

            FRAME_COUNTER => {
                self.mode = match data & 0x80 == 0 {
                    true => SequencerMode::FiveStep,
                    false => SequencerMode::FourStep,
                };

                self.frame_counter = 0;
                self.sequencer = 0;

                self.disable_interrupt = data & 0x40 != 0;

                // Clear the IRQ flag if set to disabled
                if self.disable_interrupt {
                    self.dmc.poll_interrupt();
                    self.pending_interrupt = None;
                }
            }

            _ => (),
        }
    }

    /// Returns an audio sample from the APU.
    ///
    /// The NES APU mixer takes the channel outputs and converts them to an
    /// analog audio signal.
    pub fn output(&mut self) -> f32 {
        // Approximate the audio output level within the range of 0.0 to 1.0.
        let pulse_output = 95.88
            / (100.0 + (8128.0 / (self.pulse1.output() as f32 + self.pulse2.output() as f32)));

        // TODO:
        //                                   159.79
        // tnd_out = ------------------------------------------------------------
        //                                     1
        //            ----------------------------------------------------- + 100
        //             (triangle / 8227) + (noise / 12241) + (dmc / 22638)

        // TODO: Dirty hack. Remove once other channels implemented.
        let sample = pulse_output + 0.57;

        self.filters
            .iter_mut()
            .fold(sample, |sample, filter| filter.process(sample))
    }

    /// Returns the status of the APU:
    ///
    /// IF-D NT21
    ///
    /// I: DMC Interrupt requested and clears it if set
    /// F: Apu interrupt flag and clears it if set
    /// D: 1 if DMC length counter > 0
    /// N: 1 if noise length counter > 0
    /// T: 1 if triangle length counter > 0
    /// 2: 1 if pulse 2 length counter > 0
    /// 1: 1 if pulse 1 length counter > 0
    fn status(&mut self) -> u8 {
        (self.dmc.poll_interrupt() as u8) << 7
            | (self.pending_interrupt.take().is_some() as u8) << 6
            | ((self.dmc.length_counter() > 0) as u8) << 4
            | ((self.noise.length_counter() > 0) as u8) << 3
            | ((self.tri.length_counter() > 0) as u8) << 2
            | ((self.pulse2.length_counter() > 0) as u8) << 1
            | (self.pulse1.length_counter() > 0) as u8
    }
}
