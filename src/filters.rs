pub use highpass::HighPass;
pub use lowpass::LowPass;

mod highpass;
mod lowpass;

/// Represents an audio filter.
pub trait Filter {
    fn process(&mut self, input: f32) -> f32;
}
