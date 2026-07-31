mod identity;
mod grayscale;
mod mask;

pub use identity::IdentityFilter;
pub use grayscale::GrayscaleFilter;
pub use mask::MaskFilter;

/// Trait for bitmap post-processing filters.
/// Mirrors filters/Filter.js.
pub trait BitmapFilter {
    fn apply(&self, pixels: &mut [u8]);
}

/// Get a filter by type name.
pub fn get_filter_by_type(type_name: &str) -> Option<Box<dyn BitmapFilter>> {
    match type_name.to_lowercase().as_str() {
        "none" | "identity" => Some(Box::new(IdentityFilter)),
        "grayscale" => Some(Box::new(GrayscaleFilter)),
        "mask" => Some(Box::new(MaskFilter)),
        _ => None,
    }
}
