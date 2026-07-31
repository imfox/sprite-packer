use super::BitmapFilter;

/// Identity/no-op filter. Mirrors filters/Filter.js.
pub struct IdentityFilter;

impl BitmapFilter for IdentityFilter {
    fn apply(&self, _pixels: &mut [u8]) {
        // No-op
    }
}
