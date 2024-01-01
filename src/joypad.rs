pub const JOYPAD_RIGHT: u8 = 0b10000000;
pub const JOYPAD_LEFT: u8 = 0b01000000;
pub const JOYPAD_DOWN: u8 = 0b00100000;
pub const JOYPAD_UP: u8 = 0b00010000;
pub const JOYPAD_START: u8 = 0b00001000;
pub const JOYPAD_SELECT: u8 = 0b00000100;
pub const JOYPAD_BUTTON_B: u8 = 0b00000010;
pub const JOYPAD_BUTTON_A: u8 = 0b00000001;

/// Represents a NES joypad.
///
/// NES joypads report the status of one button at a time in this order:
///
/// A -> B -> Select -> Start -> Up -> Down -> Left -> Right
///
/// After reporting the state of the button RIGHT, the controller would
/// continually return 1s for all following read, until a strobe mode change.
///
/// The controller operates in 2 modes:
///   - strobe bit on: controller reports only status of the button A on every
///     read
///   - strobe bit off: controller cycles through all buttons
pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: u8,
}

impl Joypad {
    /// Returns an instantiated joypad.
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: 0b00000000,
        }
    }

    /// Writes the status of the joypad.
    pub fn write(&mut self, data: u8) {
        self.strobe = data & 1 == 1;

        // Reset index back to A if strobe mode is on.
        if self.strobe {
            self.button_index = 0
        }
    }

    /// Returns the status of the current button.
    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }

        let response = (self.button_status & (1 << self.button_index)) >> self.button_index;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }

        response
    }

    /// Sets the pressed state of the given button.
    pub fn set_button_pressed_status(&mut self, button: u8, pressed: bool) {
        if pressed {
            self.button_status |= button;
        } else {
            self.button_status &= !button;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strobe_mode() {
        let mut joypad = Joypad::new();
        joypad.write(1);
        joypad.set_button_pressed_status(JOYPAD_BUTTON_A, true);
        for _x in 0..10 {
            assert_eq!(joypad.read(), 1);
        }
    }

    #[test]
    fn test_strobe_mode_on_off() {
        let mut joypad = Joypad::new();

        joypad.write(0);
        joypad.set_button_pressed_status(JOYPAD_RIGHT, true);
        joypad.set_button_pressed_status(JOYPAD_LEFT, true);
        joypad.set_button_pressed_status(JOYPAD_SELECT, true);
        joypad.set_button_pressed_status(JOYPAD_BUTTON_B, true);

        for _ in 0..=1 {
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);

            for _x in 0..10 {
                assert_eq!(joypad.read(), 1);
            }
            joypad.write(1);
            joypad.write(0);
        }
    }
}
