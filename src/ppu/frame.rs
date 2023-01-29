use super::palette;

/// Frame represents one rendered frame of pixels.
pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;

    /// Returns a new frame.
    pub fn new() -> Self {
        Frame {
            data: vec![0; (Frame::WIDTH) * (Frame::HEIGHT) * 3],
        }
    }

    /// Sets a pixel in the given position with the given colour.
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: &palette::Rgb) {
        let base = y * 3 * Frame::WIDTH + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }

    /// Returns the current frame contents.
    pub fn pixels(&self) -> &[u8] {
        &self.data
    }
}
