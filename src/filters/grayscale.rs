use super::BitmapFilter;

/// Grayscale filter. Mirrors filters/Grayscale.js.
pub struct GrayscaleFilter;

impl BitmapFilter for GrayscaleFilter {
    fn apply(&self, pixels: &mut [u8]) {
        for chunk in pixels.chunks_mut(4) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            let v = (0.2126 * r + 0.7152 * g + 0.0722 * b).round() as u8;
            chunk[0] = v;
            chunk[1] = v;
            chunk[2] = v;
            // alpha stays
        }
    }
}
