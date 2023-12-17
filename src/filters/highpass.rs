use super::Filter;

// Implements a basic highpass filter.
pub struct HighPass {
    alpha: f32,
    last_input: f32,
    last_output: f32,
}

impl HighPass {
    // Creates a new highpass filter.
    pub fn new(sample_rate: f32, cutoff_frequency: f32) -> Self {
        let alpha = 1.0 / (1.0 + sample_rate / (2.0 * std::f32::consts::PI * cutoff_frequency));
        Self {
            alpha,
            last_input: 0.0,
            last_output: 0.0,
        }
    }
}

impl Filter for HighPass {
    // Processes a sample through the highpass filter.
    fn process(&mut self, input: f32) -> f32 {
        // TODO: Fix this, too aggressive and filters everything.
        // self.last_output = self.alpha * (self.last_output + input - self.last_input);
        // self.last_input = input;
        // self.last_output

        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highpass_new() {
        let highpass = HighPass::new(44100.0, 1000.0);
        assert_eq!(highpass.last_input, 0.0);
        assert_eq!(highpass.last_output, 0.0);
    }

    // TODO: Uncomment when fixed.
    // #[test]
    // fn test_highpass_process() {
    //    let mut highpass = HighPass::new(44100.0, 1000.0);
    //    let output = highpass.process(100.0);
    //    assert_eq!(highpass.last_input, 100.0);
    //   assert_eq!(highpass.last_output, output);
    //
    //    let output2 = highpass.process(200.0);
    //    assert_eq!(output2, 14.026008);
    //}
}
