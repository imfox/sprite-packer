use crate::math::Rect;

/// Packing logic for the npm maxrects-packer engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpmLogic {
    MaxEdge,
    MaxArea,
}

/// The 4 methods of free-tex-packer-core's MaxRectsPacker wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpmMethod {
    Smart,
    SmartArea,
    Square,
    SquareArea,
}

impl NpmMethod {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "smart" => Some(Self::Smart),
            "smartarea" => Some(Self::SmartArea),
            "square" => Some(Self::Square),
            "squarearea" => Some(Self::SquareArea),
            _ => None,
        }
    }

    /// Mirrors the options object built in MaxRectsPacker.js pack().
    pub fn options(&self, allow_rotation: bool) -> NpmBinOptions {
        match self {
            Self::Smart => NpmBinOptions {
                smart: true,
                square: false,
                allow_rotation,
                logic: NpmLogic::MaxEdge,
            },
            Self::SmartArea => NpmBinOptions {
                smart: true,
                square: false,
                allow_rotation,
                logic: NpmLogic::MaxArea,
            },
            Self::Square => NpmBinOptions {
                smart: false,
                square: true,
                allow_rotation,
                logic: NpmLogic::MaxEdge,
            },
            Self::SquareArea => NpmBinOptions {
                smart: false,
                square: true,
                allow_rotation,
                logic: NpmLogic::MaxArea,
            },
        }
    }
}

/// Options mirroring maxrects-packer IOption for the engine.
#[derive(Debug, Clone)]
pub struct NpmBinOptions {
    pub smart: bool,
    pub square: bool,
    pub allow_rotation: bool,
    pub logic: NpmLogic,
}

/// A rect placed in an NpmBin.
#[derive(Debug, Clone)]
pub struct NpmPlaced {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub rot: bool,
}

/// Single-bin port of maxrects-packer's MaxRectsBin.
///
/// Mirrors maxrects-bin.ts exactly: single-number scoring (MAX_EDGE / MAX_AREA),
/// smart growth via updateBinSize/expandFreeRects, and the npm-style pruneFreeList
/// that re-checks elements shifted into a removed slot.
pub struct NpmBin {
    max_width: i32,
    max_height: i32,
    pub width: i32,
    pub height: i32,
    free_rects: Vec<Rect>,
    pub rects: Vec<NpmPlaced>,
    vertical_expand: bool,
    smart: bool,
    square: bool,
    allow_rotation: bool,
    logic: NpmLogic,
}

impl NpmBin {
    /// Construct with maxWidth/maxHeight and padding 0, border 0 — exactly as
    /// free-tex-packer-core does: `new MaxRectsPackerEngine(binWidth, binHeight, 0, options)`.
    pub fn new(max_width: i32, max_height: i32, options: &NpmBinOptions) -> Self {
        let (width, height) = if options.smart {
            (0, 0)
        } else {
            (max_width, max_height)
        };
        let mut bin = Self {
            max_width,
            max_height,
            width,
            height,
            free_rects: Vec::new(),
            rects: Vec::new(),
            vertical_expand: false,
            smart: options.smart,
            square: options.square,
            allow_rotation: options.allow_rotation,
            logic: options.logic,
        };
        bin.free_rects.push(Rect::new(0, 0, max_width, max_height));
        bin
    }

    /// Add a rect (w × h) with the given index. Returns true when placed.
    /// Mirrors Bin.add → place; failed placements may still grow the bin.
    pub fn add(&mut self, w: i32, h: i32, index: usize) -> bool {
        if let Some((x, y, rot)) = self.place(w, h) {
            self.rects.push(NpmPlaced { index, x, y, rot });
            true
        } else {
            false
        }
    }

    /// Oversized-element bin: a rect larger than the atlas is held without placement
    /// (mirrors OversizedElementBin). Never reached for validated inputs.
    pub fn oversized(&mut self, index: usize) {
        self.rects.push(NpmPlaced { index, x: 0, y: 0, rot: false });
    }

    fn place(&mut self, w: i32, h: i32) -> Option<(i32, i32, bool)> {
        if let Some(node) = self.find_node(w, h) {
            self.update_bin_size(&node);
            // Split loop. numRectToProcess is fixed at entry; newly split free rects are
            // appended beyond it and only seen by later placements / pruning.
            let mut num_to_process = self.free_rects.len();
            let mut i: usize = 0;
            while i < num_to_process {
                if self.split_node(self.free_rects[i], &node) {
                    self.free_rects.remove(i);
                    num_to_process -= 1;
                    // net i unchanged → recheck element shifted into slot i
                } else {
                    i += 1;
                }
            }
            self.prune_free_list();
            self.vertical_expand = self.width > self.height;
            return Some((node.x, node.y, node.rot));
        } else if !self.vertical_expand {
            let right = Node { x: self.width, y: 0, w, h, rot: false };
            let down = Node { x: 0, y: self.height, w, h, rot: false };
            if self.update_bin_size(&right) || self.update_bin_size(&down) {
                return self.place(w, h);
            }
        } else {
            let down = Node { x: 0, y: self.height, w, h, rot: false };
            let right = Node { x: self.width, y: 0, w, h, rot: false };
            if self.update_bin_size(&down) || self.update_bin_size(&right) {
                return self.place(w, h);
            }
        }
        None
    }

    fn find_node(&self, w: i32, h: i32) -> Option<Node> {
        let mut best_score = i64::MAX;
        let mut best: Option<Node> = None;
        for r in &self.free_rects {
            if r.width >= w && r.height >= h {
                let score = match self.logic {
                    NpmLogic::MaxArea => r.width as i64 * r.height as i64 - w as i64 * h as i64,
                    NpmLogic::MaxEdge => (r.width - w).min(r.height - h) as i64,
                };
                if score < best_score {
                    best = Some(Node { x: r.x, y: r.y, w, h, rot: false });
                    best_score = score;
                }
            }
            if !self.allow_rotation {
                continue;
            }
            if r.width >= h && r.height >= w {
                let score = match self.logic {
                    NpmLogic::MaxArea => r.width as i64 * r.height as i64 - h as i64 * w as i64,
                    NpmLogic::MaxEdge => (r.height - w).min(r.width - h) as i64,
                };
                if score < best_score {
                    best = Some(Node { x: r.x, y: r.y, w: h, h: w, rot: true });
                    best_score = score;
                }
            }
        }
        best
    }

    fn update_bin_size(&mut self, node: &Node) -> bool {
        if !self.smart {
            return false;
        }
        // stage.contain(node) — node.x/y are always >= 0, so this reduces to "fits the
        // current width×height". The inclusive bound mirrors contain().
        if node.x + node.w <= self.width && node.y + node.h <= self.height {
            return false;
        }
        let mut tmp_w = self.width.max(node.x + node.w);
        let mut tmp_h = self.height.max(node.y + node.h);
        let tmp_fits = tmp_w <= self.max_width && tmp_h <= self.max_height;
        if self.allow_rotation {
            let rot_w = self.width.max(node.x + node.h);
            let rot_h = self.height.max(node.y + node.w);
            let rot_fits = rot_w <= self.max_width && rot_h <= self.max_height;
            if tmp_fits && rot_fits && rot_w * rot_h < tmp_w * tmp_h {
                tmp_w = rot_w;
                tmp_h = rot_h;
            }
            if rot_fits && !tmp_fits {
                tmp_w = rot_w;
                tmp_h = rot_h;
            }
        }
        if self.square {
            tmp_w = tmp_w.max(tmp_h);
            tmp_h = tmp_w;
        }
        if tmp_w > self.max_width || tmp_h > self.max_height {
            return false;
        }
        // expandFreeRects uses the OLD width/height to position the new strips
        self.expand_free_rects(tmp_w, tmp_h);
        self.width = tmp_w;
        self.height = tmp_h;
        true
    }

    fn expand_free_rects(&mut self, width: i32, height: i32) {
        for free in &mut self.free_rects {
            if free.x + free.width >= self.width.min(width) {
                free.width = width - free.x;
            }
            if free.y + free.height >= self.height.min(height) {
                free.height = height - free.y;
            }
        }
        // Right strip and bottom strip (padding 0, border 0)
        self.free_rects.push(Rect::new(self.width, 0, width - self.width, height));
        self.free_rects.push(Rect::new(0, self.height, width, height - self.height));
        self.free_rects
            .retain(|r| r.width > 0 && r.height > 0 && r.x >= 0 && r.y >= 0);
        self.prune_free_list();
    }

    fn split_node(&mut self, free: Rect, used: &Node) -> bool {
        // collide(): strict intersection
        if !(free.x < used.x + used.w
            && free.x + free.width > used.x
            && free.y < used.y + used.h
            && free.y + free.height > used.y)
        {
            return false;
        }
        // Vertical split
        if used.x < free.x + free.width && used.x + used.w > free.x {
            if used.y > free.y && used.y < free.y + free.height {
                self.free_rects.push(Rect::new(free.x, free.y, free.width, used.y - free.y));
            }
            if used.y + used.h < free.y + free.height {
                self.free_rects.push(Rect::new(
                    free.x,
                    used.y + used.h,
                    free.width,
                    free.y + free.height - (used.y + used.h),
                ));
            }
        }
        // Horizontal split
        if used.y < free.y + free.height && used.y + used.h > free.y {
            if used.x > free.x && used.x < free.x + free.width {
                self.free_rects.push(Rect::new(free.x, free.y, used.x - free.x, free.height));
            }
            if used.x + used.w < free.x + free.width {
                self.free_rects.push(Rect::new(
                    used.x + used.w,
                    free.y,
                    free.x + free.width - (used.x + used.w),
                    free.height,
                ));
            }
        }
        true
    }

    /// Mirrors maxrects-packer's pruneFreeList: `len` is captured once and each splice
    /// decrements it; a removed element at i is re-checked (i-- then i++ net to i).
    fn prune_free_list(&mut self) {
        let mut i: i64 = 0;
        let mut len: i64 = self.free_rects.len() as i64;
        while i < len {
            let mut j = i + 1;
            while j < len {
                let a = self.free_rects[i as usize];
                let b = self.free_rects[j as usize];
                if b.contains(&a) {
                    self.free_rects.remove(i as usize);
                    i -= 1;
                    len -= 1;
                    break;
                }
                if a.contains(&b) {
                    self.free_rects.remove(j as usize);
                    j -= 1;
                    len -= 1;
                }
                j += 1;
            }
            i += 1;
        }
    }
}

/// A node placed in a bin (position + possibly-swapped dims + rotation flag).
#[derive(Debug, Clone, Copy)]
struct Node {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rot: bool,
}

/// Multi-bin port of maxrects-packer's MaxRectsPacker.
///
/// Only the non-tag path of addArray is needed (free-tex-packer-core never tags):
/// rects are sorted by logic then added in order, each rect going to the first bin
/// that accepts it, or a fresh bin otherwise.
pub struct NpmPacker {
    pub bins: Vec<NpmBin>,
    width: i32,
    height: i32,
    allow_rotation: bool,
    options: NpmBinOptions,
    current_bin_index: usize,
}

impl NpmPacker {
    pub fn new(width: i32, height: i32, options: NpmBinOptions) -> Self {
        let allow_rotation = options.allow_rotation;
        Self {
            bins: Vec::new(),
            width,
            height,
            allow_rotation,
            options,
            current_bin_index: 0,
        }
    }

    /// Sort and add — mirrors MaxRectsPacker.addArray (no-tag path). The input slice
    /// is reordered in place; use the sorted order only via the resulting bins.
    pub fn add_array(&mut self, rects: &mut [(i32, i32, usize)]) {
        let logic = self.options.logic;
        rects.sort_by(|a, b| {
            match logic {
                NpmLogic::MaxEdge => b.0.max(b.1).cmp(&a.0.max(a.1)),
                NpmLogic::MaxArea => (b.0 * b.1).cmp(&(a.0 * a.1)),
            }
        });
        let snapshot: Vec<(i32, i32, usize)> = rects.to_vec();
        for &(w, h, index) in &snapshot {
            self.add(w, h, index);
        }
    }

    fn add(&mut self, w: i32, h: i32, index: usize) {
        let fits = (w <= self.width && h <= self.height)
            || (self.allow_rotation && w <= self.height && h <= self.width);
        if !fits {
            self.bins.push(NpmBin::oversized_bin(index));
            return;
        }
        let mut placed = false;
        for bin in self.bins.iter_mut().skip(self.current_bin_index) {
            if bin.add(w, h, index) {
                placed = true;
                break;
            }
        }
        if !placed {
            let mut bin = NpmBin::new(self.width, self.height, &self.options);
            bin.add(w, h, index);
            self.bins.push(bin);
        }
    }
}

impl NpmBin {
    fn oversized_bin(index: usize) -> Self {
        let mut bin = NpmBin::new(0, 0, &NpmBinOptions {
            smart: false,
            square: false,
            allow_rotation: false,
            logic: NpmLogic::MaxEdge,
        });
        bin.oversized(index);
        bin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smart_area(max_w: i32, max_h: i32) -> NpmPacker {
        NpmPacker::new(
            max_w,
            max_h,
            NpmMethod::SmartArea.options(false),
        )
    }

    fn rects(items: &[(i32, i32)]) -> Vec<(i32, i32, usize)> {
        items
            .iter()
            .enumerate()
            .map(|(i, &(w, h))| (w, h, i))
            .collect()
    }

    #[test]
    fn test_npm_place_simple() {
        let mut packer = smart_area(256, 256);
        let mut input = rects(&[(100, 100), (100, 100)]);
        packer.add_array(&mut input);
        let sheet = &packer.bins[0].rects;
        assert_eq!(sheet.len(), 2);
        let a = &sheet[0];
        let b = &sheet[1];
        // no overlap
        let ra = Rect::new(a.x, a.y, 100, 100);
        let rb = Rect::new(b.x, b.y, 100, 100);
        assert!(!(ra.x < rb.x + rb.width && ra.x + ra.width > rb.x && ra.y < rb.y + rb.height && ra.y + ra.height > rb.y));
    }

    #[test]
    fn test_npm_multi_bin_overflow() {
        // Two rects too big to share one 128x128 bin → second goes to bins[1]
        let mut packer = smart_area(128, 128);
        let mut input = rects(&[(100, 100), (100, 100)]);
        packer.add_array(&mut input);
        assert_eq!(packer.bins.len(), 2);
        assert_eq!(packer.bins[0].rects.len(), 1);
        assert_eq!(packer.bins[1].rects.len(), 1);
    }

    #[test]
    fn test_npm_square_logic() {
        // Square: bin starts full-size, no smart growth
        let mut packer = NpmPacker::new(
            256,
            256,
            NpmMethod::Square.options(false),
        );
        assert_eq!(packer.bins.len(), 0);
        let mut input = rects(&[(50, 30)]);
        packer.add_array(&mut input);
        assert_eq!(packer.bins.len(), 1);
        assert_eq!(packer.bins[0].rects.len(), 1);
    }
}
