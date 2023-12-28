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

/// Triangle channel registers.
const TRIANGLE_LINEAR: u16 = 0x4008;
const TRIANGLE_TIMER_LOW: u16 = 0x400A;
const TRIANGLE_TIMER_HIGH: u16 = 0x400B;

/// Noise channel registers.
const NOISE_VOLUME: u16 = 0x400C;
const NOISE_TIMER_LOW: u16 = 0x400E;
const NOISE_TIMER_HIGH: u16 = 0x400F;

/// DMC frequency registers.
const DMC_SAMPLE_FREQUENCY: u16 = 0x4010;
const DMC_SAMPLE_RAW: u16 = 0x4011;
const DMC_SAMPLE_START: u16 = 0x4012;
const DMC_SAMPLE_LENGTH: u16 = 0x4013;

/// Sound status / enable register
const STATUS_REGISTER: u16 = 0x4015;

/// Frame counter register
const FRAME_COUNTER: u16 = 0x4017;

use dmc::Dmc;
use noise::Noise;
use pulse::Pulse;
use triangle::Triangle;

use crate::filters::{Filter, HighPass, LowPass};

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
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,

    pulse_table: [f32; 31],
    tnd_table: [f32; 203],

    filters: Vec<Box<dyn Filter>>,
}

impl Apu {
    /// Creates a new APU.
    pub fn new(sample_rate: f32) -> Self {
        let mut apu = Apu {
            cycles: 0,
            frame_counter: 0,
            disable_interrupt: false,
            pending_interrupt: None,

            sequencer: 0,
            mode: SequencerMode::FourStep,

            pulse1: Pulse::new(),
            pulse2: Pulse::new(),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),

            pulse_table: [0.0; 31],
            tnd_table: [0.0; 203],

            filters: vec![
                Box::new(HighPass::new(90.0, sample_rate)),
                Box::new(HighPass::new(440.0, sample_rate)),
                Box::new(LowPass::new(14000.0, sample_rate)),
            ],
        };

        // Precompute the pulse and tnd lookup tables.
        //
        // See: https://www.nesdev.org/wiki/APU_Mixer#Emulation
        for i in 0..31 {
            apu.pulse_table[i] = 95.52 / (8128.0 / i as f32 + 100.0);
        }
        for i in 0..203 {
            apu.tnd_table[i] = 163.67 / (24329.0 / i as f32 + 100.0);
        }

        apu
    }

    /// Advances the state of the APU by one CPU cycle.
    pub fn clock(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);

        self.triangle.clock_timer();
        self.dmc.clock();

        // Pulse and noise channels are clocked at half the rate of the CPU.
        if self.cycles % 2 == 0 {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
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

            // Sweep and length clocks.
            if (self.frame_counter & 0x5) == 1 {
                self.pulse1.clock_length();
                self.pulse2.clock_length();
                self.pulse1.clock_sweep(pulse::Channel::One);
                self.pulse2.clock_sweep(pulse::Channel::Two);
                self.triangle.clock_length();
                self.noise.clock_length();
            }

            if self.sequencer < 4 {
                self.pulse1.clock_envelope();
                self.pulse2.clock_envelope();
                self.noise.clock_envelope();
                self.triangle.clock_counter();
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

            TRIANGLE_LINEAR => self.triangle.write_linear_counter(data),
            TRIANGLE_TIMER_LOW => self.triangle.write_timer_low(data),
            TRIANGLE_TIMER_HIGH => self.triangle.write_timer_high(data),

            NOISE_VOLUME => self.noise.write_volume(data),
            NOISE_TIMER_LOW => self.noise.write_timer_low(data),
            NOISE_TIMER_HIGH => self.noise.write_timer_high(data),

            DMC_SAMPLE_FREQUENCY => self.dmc.write_sample_frequency(data),
            DMC_SAMPLE_RAW => self.dmc.write_raw_sample(data),
            DMC_SAMPLE_START => self.dmc.write_sample_start(data),
            DMC_SAMPLE_LENGTH => self.dmc.write_sample_length(data),

            // ---D NT21
            // D: Enable DMC
            // N: Noise
            // T: Triangle
            // 2: Pulse channel 2
            // 1: Pulse channel 1
            STATUS_REGISTER => {
                self.pulse1.toggle(data & 0x1 != 0);
                self.pulse2.toggle(data & 0x2 != 0);
                self.triangle.toggle(data & 0x4 != 0);
                self.noise.toggle(data & 0x8 != 0);
                self.dmc.toggle(data & 0x10 != 0);
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
        // The APU mixer formulas can be efficiently implemented using lookup
        // tables.
        //
        // See: https://www.nesdev.org/wiki/APU_Mixer#Emulation
        let pulse_output = self.pulse_table[(self.pulse1.output() + self.pulse2.output()) as usize];

        let tnd_output = self.tnd_table
            [(3 * self.triangle.output() + 2 * self.noise.output() + self.dmc.output()) as usize];

        let sample = pulse_output + tnd_output;

        self.filters
            .iter_mut()
            .fold(sample, |sample, filter| filter.process(sample))
    }

    /// Polls the IRQ flag
    pub fn poll_interrupt(&mut self) -> bool {
        self.pending_interrupt.take().is_some() | self.dmc.poll_interrupt()
    }

    /// Returns true if the DMC needs a new sample.
    pub fn need_dmc_sample(&mut self) -> bool {
        self.dmc.need_sample()
    }

    /// Sets the sample for the DMC.
    pub fn set_dmc_sample(&mut self, sample: u8) {
        self.dmc.set_sample(sample);
    }

    /// Gets the address of the next DMC audio sample.
    pub fn dmc_sample_address(&self) -> u16 {
        self.dmc.address()
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
            | ((self.triangle.length_counter() > 0) as u8) << 2
            | ((self.pulse2.length_counter() > 0) as u8) << 1
            | (self.pulse1.length_counter() > 0) as u8
    }
}
