/// Represents the NES delta modulation channel (DMC) which can output 1-bit
/// delta-encoded samples or can have its 7-bit counter directly loaded,
/// allowing flexible manual sample playback.
pub struct Dmc {
    disable_interrupt: bool,
    pending_interrupt: Option<bool>,

    length_counter: u8,
}

impl Dmc {
    /// Creates a new DMC.
    pub fn new() -> Self {
        Self {
            length_counter: 0,
            disable_interrupt: false,
            pending_interrupt: None,
        }
    }

    /// Resets the DMC.
    pub fn reset(&mut self) {
        self.length_counter = 0;
    }

    /// Returns the length counter value
    pub fn length_counter(&self) -> u8 {
        self.length_counter
    }

    /// Returns true if the DMC chanel is waiting for an interrupt.
    pub fn poll_interrupt(&mut self) -> bool {
        self.pending_interrupt.take().is_some()
    }
}
