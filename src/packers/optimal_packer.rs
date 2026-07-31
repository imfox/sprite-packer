/// Marker type for the optimal packer — mirrors free-tex-packer-core's OptimalPacker.
///
/// It does not pack anything itself (pack() throws in the JS original). When selected,
/// PackProcessor enumerates every packer-method/rotation combination and keeps the result
/// with the fewest sheets, then the highest efficiency.
pub struct OptimalPacker;

impl OptimalPacker {
    pub const TYPE: &'static str = "OptimalPacker";
}
