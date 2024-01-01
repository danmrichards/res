use std::f32::consts::PI;

/// Represents a filter that processs an audio sample.
pub trait Filter {
    fn process(&mut self, sample: f32) -> f32;
}

/// Represents a high-pass filter that passes signals with a frequency higher
/// than a certain cutoff frequency and attenuates signals with frequencies
/// lower than the cutoff frequency.
///
/// See: https://en.wikipedia.org/wiki/High-pass_filter
pub struct HighPass {
    a: f32,
    prev_input: f32,
    prev_output: f32,
}

impl HighPass {
    /// Returns a new HighPass filter.
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        let rc = calc_time_constant(freq);
        let dt = calc_time_interval(sample_rate);

        HighPass {
            a: rc / (rc + dt),
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }
}

impl Filter for HighPass {
    /// Processes an audio sample.
    fn process(&mut self, input: f32) -> f32 {
        let output = self.a * self.prev_output + self.a * (input - self.prev_input);
        self.prev_input = input;
        self.prev_output = output;

        output
    }
}

/// Represents a low-pass filter that passes signals with a frequency lower than
/// a selected cutoff frequency and attenuates signals with frequencies higher
/// than the cutoff frequency.
///
/// See: https://en.wikipedia.org/wiki/Low-pass_filter
pub struct LowPass {
    a: f32,
    prev_input: f32,
    prev_output: f32,
}

impl LowPass {
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        let rc = calc_time_constant(freq);
        let dt = calc_time_interval(sample_rate);

        LowPass {
            a: dt / (rc + dt),
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }
}

impl Filter for LowPass {
    /// Processes an audio sample.
    fn process(&mut self, input: f32) -> f32 {
        let output = self.a * input + (1.0 - self.a) * self.prev_output;
        self.prev_input = input;
        self.prev_output = output;

        output
    }
}

/// Returns the time constant based on the given frequency.
fn calc_time_constant(freq: f32) -> f32 {
    1.0 / (2.0 * PI * freq)
}

/// Returns the time interval based on the given sample rate.
fn calc_time_interval(sample_rate: f32) -> f32 {
    1.0 / sample_rate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_pass_filter() {
        let mut filter = HighPass::new(440.0, 44100.0);
        let input = 0.5;
        let output = filter.process(input);
        assert!(output >= 0.0);
        assert_eq!(filter.prev_input, input);
        assert_eq!(filter.prev_output, output);
    }

    #[test]
    fn test_low_pass_filter() {
        let mut filter = LowPass::new(440.0, 44100.0);
        let input = 0.5;
        let output = filter.process(input);
        assert!(output >= 0.0);
        assert_eq!(filter.prev_input, input);
        assert_eq!(filter.prev_output, output);
    }

    #[test]
    fn test_calc_time_constant() {
        let freq = 440.0;
        let expected = 1.0 / (2.0 * PI * freq);
        let result = calc_time_constant(freq);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calc_time_interval() {
        let sample_rate = 44100.0;
        let expected = 1.0 / sample_rate;
        let result = calc_time_interval(sample_rate);
        assert_eq!(result, expected);
    }
}
