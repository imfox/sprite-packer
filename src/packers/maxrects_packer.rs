use super::maxrects_bin::{MaxRectsBin, BinOptions, PackMethod};
use super::PackInput;

/// Multi-bin manager wrapping MaxRectsBin — mirrors maxrects-packer's MaxRectsPacker.
///
/// Automatically creates new bins when rects don't fit existing ones.
/// Use `add_array()` to pack a batch of rects, then iterate `bins` for per-sheet results.
pub struct MaxRectsPacker {
    pub bins: Vec<MaxRectsBin>,
    pub width: i32,
    pub height: i32,
    pub padding: i32,
    pub options: BinOptions,
    current_bin_index: usize,
}

impl MaxRectsPacker {
    pub fn new(width: i32, height: i32, padding: i32, options: BinOptions) -> Self {
        Self {
            bins: Vec::new(),
            width,
            height,
            padding,
            options,
            current_bin_index: 0,
        }
    }

    /// Add a single rect. Tries existing bins (from current_bin_index), creates a new bin if needed.
    pub fn add(&mut self, input: &PackInput, method: PackMethod) -> Option<(i32, i32, bool)> {
        // Try existing bins
        for bin in self.bins.iter_mut().skip(self.current_bin_index) {
            if let Some(output) = bin.place(input.width, input.height, method) {
                return Some((output.x, output.y, output.rotated));
            }
        }

        // Create new bin
        let mut bin = MaxRectsBin::new(
            self.width,
            self.height,
            self.padding,
            self.options.allow_rotation,
            &self.options,
        );
        if let Some(output) = bin.place(input.width, input.height, method) {
            self.bins.push(bin);
            Some((output.x, output.y, output.rotated))
        } else {
            None
        }
    }

    /// Add an array of rects, sorted by area descending.
    pub fn add_array(&mut self, inputs: &[PackInput], method: PackMethod) {
        let mut sorted: Vec<&PackInput> = inputs.iter().collect();
        sorted.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));
        for input in sorted {
            self.add(input, method);
        }
    }

    /// Lock current bins — new rects won't be placed into existing bins.
    pub fn next(&mut self) {
        self.current_bin_index = self.bins.len();
    }

    /// Reset all bins but keep settings.
    pub fn reset(&mut self) {
        self.bins.clear();
        self.current_bin_index = 0;
    }
}
