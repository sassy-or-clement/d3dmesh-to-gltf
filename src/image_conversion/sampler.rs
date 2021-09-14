use image::{ImageBuffer, Luma};
pub struct Linear {
    image: ImageBuffer<Luma<f32>, Vec<f32>>,
}
impl Linear {
    pub fn new(image: ImageBuffer<Luma<f32>, Vec<f32>>) -> Self {
        Self { image }
    }
    /// Simple linear (bilinear) interpolation of the given position in the image
    pub fn get_pixel(&self, x: f32, y: f32) -> f32 {
        let dx = x.fract();
        let dy = y.fract();
        let c00 = self.image.get_pixel(x.floor() as u32, y.floor() as u32)[0];
        let c10 = self.image.get_pixel(x.ceil() as u32, y.floor() as u32)[0];
        let c01 = self.image.get_pixel(x.floor() as u32, y.ceil() as u32)[0];
        let c11 = self.image.get_pixel(x.ceil() as u32, y.ceil() as u32)[0];
        let a = c00 * (1.0 - dx) + c10 * dx;
        let b = c01 * (1.0 - dx) + c11 * dx;
        (a * (1.0 - dy)) + (b * dy)
    }
}
