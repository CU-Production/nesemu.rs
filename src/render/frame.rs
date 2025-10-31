pub struct Frame {
    pub data: Vec<u32>,
}

impl Frame {
    pub const WIDTH: usize = 256;
    pub const HEIGHT: usize = 240;

    pub fn new() -> Self {
        Frame {
            data: vec![0; Frame::WIDTH * Frame::HEIGHT],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let index = y * Frame::WIDTH + x;
        if index < self.data.len() {
            // Convert RGB to u32 format (0RGB)
            self.data[index] = ((rgb.0 as u32) << 16) | ((rgb.1 as u32) << 8) | (rgb.2 as u32);
        }
    }
}
