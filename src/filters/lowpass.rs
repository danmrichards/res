use super::Filter;

// Implements a basic lowpass filter.
pub struct LowPass {
    alpha: f32,
    last_output: f32,
}

impl LowPass {
    // Creates a new lowpass filter.
    pub fn new(sample_rate: f32, cutoff_frequency: f32) -> Self {
        let alpha = 1.0 / (1.0 + sample_rate / (2.0 * std::f32::consts::PI * cutoff_frequency));
        Self {
            alpha,
            last_output: 0.0,
        }
    }
}

impl Filter for LowPass {
    fn process(&mut self, input: f32) -> f32 {
        self.last_output = self.alpha * input + (1.0 - self.alpha) * self.last_output;
        self.last_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowpass_new() {
        let sample_rate = 44100.0;
        let cutoff_frequency = 1000.0;
        let filter = LowPass::new(sample_rate, cutoff_frequency);

        assert_eq!(filter.alpha, 0.124_708);
        assert_eq!(filter.last_output, 0.0);
    }

    #[test]
    fn test_lowpass_process() {
        let sample_rate = 44100.0;
        let cutoff_frequency = 1000.0;
        let mut filter = LowPass::new(sample_rate, cutoff_frequency);
        let input = 0.5;

        let output = filter.process(input);

        assert_eq!(output, 0.062_354);
        assert_eq!(filter.last_output, output);
    }
}
