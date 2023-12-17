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
    // Processes a sample through the lowpass filter.
    //
    // The output of the filter is a weighted average of the current input and
    // the previous output. The weights are determined by the alpha factor,
    // which depends on the cutoff frequency of the filter. The lower the cutoff
    // frequency, the more the output depends on the previous outputs and the
    // less it depends on the current input, which is how the filter achieves
    // its low-pass effect
    fn process(&mut self, input: f32) -> f32 {
        self.last_output = self.alpha * input + (1.0 - self.alpha) * self.last_output;
        self.last_output
    }

    // Resets the lowpass filter.
    fn reset(&mut self) {
        self.last_output = 0.0;
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

        assert_eq!(filter.alpha, 0.124707997);
        assert_eq!(filter.last_output, 0.0);
    }

    #[test]
    fn test_lowpass_process() {
        let sample_rate = 44100.0;
        let cutoff_frequency = 1000.0;
        let mut filter = LowPass::new(sample_rate, cutoff_frequency);
        let input = 0.5;

        let output = filter.process(input);

        assert_eq!(output, 0.0623539984);
        assert_eq!(filter.last_output, output);
    }

    #[test]
    fn test_lowpass_reset() {
        let sample_rate = 44100.0;
        let cutoff_frequency = 1000.0;
        let mut filter = LowPass::new(sample_rate, cutoff_frequency);

        filter.process(0.5);
        filter.reset();

        assert_eq!(filter.last_output, 0.0);
    }
}
