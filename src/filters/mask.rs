use super::BitmapFilter;

/// Binary mask filter. Mirrors filters/Mask.js.
pub struct MaskFilter;

impl BitmapFilter for MaskFilter {
    fn apply(&self, pixels: &mut [u8]) {
        for chunk in pixels.chunks_mut(4) {
            if chunk[3] == 0 {
                chunk[0] = 0;
                chunk[1] = 0;
                chunk[2] = 0;
            } else {
                chunk[0] = 255;
                chunk[1] = 255;
                chunk[2] = 255;
            }
        }
    }
}
