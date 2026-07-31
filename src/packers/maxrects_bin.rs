use crate::math::Rect;

/// Packing heuristic methods — extends free-tex-packer-core's 5 methods
/// with FILL_WIDTH from maxrects-packer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackMethod {
    BestShortSideFit,
    BestLongSideFit,
    BestAreaFit,
    BottomLeftRule,
    ContactPointRule,
    FillWidth,
}

/// Definition of a rect to be placed.
#[derive(Debug, Clone)]
pub struct PackInput {
    pub width: i32,
    pub height: i32,
    pub index: usize,
}

/// Result of placing a single rect.
#[derive(Debug, Clone)]
pub struct PackOutput {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub rotated: bool,
}

/// Options for bin construction — mirrors maxrects-packer IOption.
#[derive(Debug, Clone)]
pub struct BinOptions {
    pub smart: bool,
    pub pot: bool,
    pub square: bool,
    pub allow_rotation: bool,
    pub border: i32,
    pub logic: PackMethod,
}

impl Default for BinOptions {
    fn default() -> Self {
        Self {
            smart: true,
            pot: false,
            square: false,
            allow_rotation: false,
            border: 0,
            logic: PackMethod::BestShortSideFit,
        }
    }
}

/// MaxRectsBin — single-bin MaxRects packing algorithm.
/// Integrates smart sizing (from maxrects-packer), FILL_WIDTH, and automatic bin growth.
pub struct MaxRectsBin {
    pub max_width: i32,
    pub max_height: i32,
    pub width: i32,
    pub height: i32,
    pub allow_rotate: bool,
    pub padding: i32,
    pub border: i32,
    pub smart: bool,
    pub pot: bool,
    pub square: bool,
    used_rectangles: Vec<Rect>,
    free_rectangles: Vec<Rect>,
    // Preferred expansion direction: false = horizontal, true = vertical (mirrors maxrects-packer)
    vertical_expand: bool,
}

impl MaxRectsBin {
    pub fn new(
        max_width: i32,
        max_height: i32,
        padding: i32,
        allow_rotate: bool,
        options: &BinOptions,
    ) -> Self {
        let border = options.border;
        let start_w = if options.smart {
            0
        } else {
            max_width + padding - border * 2
        };
        let start_h = if options.smart {
            0
        } else {
            max_height + padding - border * 2
        };

        let mut bin = Self {
            max_width,
            max_height,
            width: start_w,
            height: start_h,
            allow_rotate,
            padding,
            border,
            smart: options.smart,
            pot: options.pot,
            square: options.square,
            used_rectangles: Vec::new(),
            free_rectangles: Vec::new(),
            vertical_expand: false,
        };

        bin.free_rectangles.push(Rect::new(
            border,
            border,
            max_width + padding - border * 2,
            max_height + padding - border * 2,
        ));

        bin
    }

    /// Try to place a rect of (w, h). Returns position and rotation if successful.
    /// On failure, tries to grow the bin (if smart mode) and retries.
    pub fn place(&mut self, w: i32, h: i32, method: PackMethod) -> Option<PackOutput> {
        // Step 1: find optimal node
        if let Some((_s1, _s2, x, y, pw, ph, rotated)) = self.find_node(w, h, method) {
            // Step 2: update bin size if needed (return value ignored — placement proceeds)
            if self.smart {
                self.update_bin_size(x, y, pw, ph, rotated);
            }

            // Step 3: place the node (split free rects, prune)
            let node = Rect::new(x, y, pw, ph);
            self.place_rectangle(&node);

            // Step 4: determine next expand direction
            self.vertical_expand = if method == PackMethod::FillWidth {
                false
            } else {
                self.width > self.height
            };

            return Some(PackOutput {
                index: 0, // caller sets this
                x,
                y,
                width: pw,
                height: ph,
                rotated,
            });
        }

        // Step 5: try growing the bin (maxrects-packer style).
        // Expand in the preferred direction first: right (horizontal) or down (vertical).
        if self.smart {
            let right = (self.width + self.padding - self.border, self.border);
            let down = (self.border, self.height + self.padding - self.border);
            let (first, second) = if self.vertical_expand { (down, right) } else { (right, down) };

            if self.update_bin_size(first.0, first.1, w, h, false) {
                return self.place(w, h, method);
            }
            if self.update_bin_size(second.0, second.1, w, h, false) {
                return self.place(w, h, method);
            }
        }

        None
    }

    /// Find the best position for a rect.
    /// Returns `(score1, score2, x, y, pw, ph, rotated)` so callers can run a
    /// global best-first selection (JS `insert2`).
    pub fn find_node(
        &self,
        w: i32,
        h: i32,
        method: PackMethod,
    ) -> Option<(i32, i32, i32, i32, i32, i32, bool)> {
        if method == PackMethod::ContactPointRule {
            // Mirrors free-tex-packer-core's _findPositionForNewNodeContactPoint, which
            // only ever accepts the first free rect (in list order) that fits — a JS quirk
            // (bestContactScore is clobbered from an object to a number on first hit).
            // Every fitting rect scores (1, 0), so the global loop keeps the first index.
            for free in &self.free_rectangles {
                if free.width >= w && free.height >= h {
                    return Some((1, 0, free.x, free.y, w, h, false));
                }
                if self.allow_rotate && free.width >= h && free.height >= w {
                    return Some((1, 0, free.x, free.y, h, w, true));
                }
            }
            return None;
        }

        if method == PackMethod::BestAreaFit {
            // JS quirk: the non-rotated branch does `bestAreaFit = areaFit` which rebinds
            // the local score object to a number, so after the first non-rotated selection
            // every later comparison fails — the first non-rotated-fitting free rect wins.
            // The rotated branch uses `bestAreaFit.value = areaFit` (the shared object), so
            // a rotated selection keeps proper area-min selection going. Caller-visible
            // score1 is the last rotated area (or Infinity if none — the non-rotated branch
            // never writes the shared score1); score2 is the short-side fit.
            let mut clobbered = false;
            let mut last_rotated_area: Option<i32> = None;
            let mut best_area = i32::MAX;
            let mut best_short = i32::MAX;
            let mut res: Option<(i32, i32, i32, i32, bool)> = None;
            for free in &self.free_rectangles {
                let area = free.width * free.height - w * h;
                if free.width >= w && free.height >= h {
                    let short = (free.width - w).abs().min((free.height - h).abs());
                    let take = if clobbered {
                        false
                    } else if res.is_none() {
                        true
                    } else {
                        area < best_area || (area == best_area && short < best_short)
                    };
                    if take {
                        res = Some((free.x, free.y, w, h, false));
                        best_short = short;
                        best_area = area;
                        clobbered = true;
                    }
                }
                if self.allow_rotate && free.width >= h && free.height >= w {
                    let short = (free.width - h).abs().min((free.height - w).abs());
                    let take = if clobbered {
                        false
                    } else if res.is_none() {
                        true
                    } else {
                        area < best_area || (area == best_area && short < best_short)
                    };
                    if take {
                        res = Some((free.x, free.y, h, w, true));
                        best_short = short;
                        best_area = area;
                        last_rotated_area = Some(area);
                    }
                }
            }
            return match res {
                Some((x, y, pw, ph, rot)) => {
                    Some((last_rotated_area.unwrap_or(i32::MAX), best_short, x, y, pw, ph, rot))
                }
                None => None,
            };
        }

        let mut best_x = 0i32;
        let mut best_y = 0i32;
        let mut best_pw = 0i32;
        let mut best_ph = 0i32;
        let mut best_rot = false;
        let mut best_s1 = i32::MAX;
        let mut best_s2 = i32::MAX;

        for free in &self.free_rectangles {
            // Normal
            if free.width >= w && free.height >= h {
                let (s1, s2) = self.score_free_rect(free, w, h, method, false);
                if s1 < best_s1 || (s1 == best_s1 && s2 < best_s2) {
                    best_s1 = s1;
                    best_s2 = s2;
                    best_x = free.x;
                    best_y = free.y;
                    best_pw = w;
                    best_ph = h;
                    best_rot = false;
                }
            }
            // Rotated
            if self.allow_rotate && free.width >= h && free.height >= w {
                let (s1, s2) = self.score_free_rect(free, h, w, method, true);
                if s1 < best_s1 || (s1 == best_s1 && s2 < best_s2) {
                    best_s1 = s1;
                    best_s2 = s2;
                    best_x = free.x;
                    best_y = free.y;
                    best_pw = h;
                    best_ph = w;
                    best_rot = true;
                }
            }
        }

        if best_s1 == i32::MAX {
            None
        } else {
            Some((best_s1, best_s2, best_x, best_y, best_pw, best_ph, best_rot))
        }
    }

    /// Score a free rect for placement. Returns (score1, score2).
    fn score_free_rect(
        &self,
        free: &Rect,
        pw: i32,
        ph: i32,
        method: PackMethod,
        _rotated: bool,
    ) -> (i32, i32) {
        match method {
            PackMethod::BestShortSideFit => {
                let left_h = (free.width - pw).abs();
                let left_v = (free.height - ph).abs();
                (left_h.min(left_v), left_h.max(left_v))
            }
            PackMethod::BestLongSideFit => {
                let left_h = (free.width - pw).abs();
                let left_v = (free.height - ph).abs();
                (left_h.max(left_v), left_h.min(left_v))
            }
            PackMethod::BestAreaFit => {
                let area = free.width * free.height - pw * ph;
                let short = (free.width - pw).abs().min((free.height - ph).abs());
                (area, short)
            }
            PackMethod::BottomLeftRule => {
                let top = free.y + ph;
                (top, free.x)
            }
            PackMethod::ContactPointRule => {
                let score = self.contact_point_score(free.x, free.y, pw, ph);
                (-score, 0)
            }
            PackMethod::FillWidth => {
                // FILL_WIDTH: prefer placements that fill left-to-right, top-to-bottom
                let pos_score = free.x + free.y * self.max_width.max(1);
                let height_gain = (free.y + ph - self.height).max(0);
                (pos_score + height_gain, free.x)
            }
        }
    }

    /// Update bin size when a rect extends beyond current bounds.
    /// Mirrors maxrects-packer's updateBinSize().
    fn update_bin_size(
        &mut self,
        x: i32,
        y: i32,
        pw: i32,
        ph: i32,
        rotated: bool,
    ) -> bool {
        if !self.smart {
            return false;
        }

        let (fw, fh) = if rotated { (ph, pw) } else { (pw, ph) };
        let right = x + fw - self.padding + self.border;
        let bottom = y + fh - self.padding + self.border;

        // If the node already fits within the current stage, no resize needed.
        // This also guards against infinite recursion in the growth fallback.
        if right <= self.width && bottom <= self.height {
            return false;
        }

        let mut tmp_w = self.width.max(right);
        let mut tmp_h = self.height.max(bottom);

        if self.pot {
            tmp_w = next_pow2(tmp_w as u32) as i32;
            tmp_h = next_pow2(tmp_h as u32) as i32;
        }
        if self.square {
            tmp_w = tmp_w.max(tmp_h);
            tmp_h = tmp_w;
        }

        if tmp_w > self.max_width || tmp_h > self.max_height {
            return false;
        }

        if tmp_w != self.width || tmp_h != self.height {
            // Expand BEFORE updating width/height so strips are positioned from the old size
            self.expand_free_rects(tmp_w + self.padding, tmp_h + self.padding);
            self.width = tmp_w;
            self.height = tmp_h;
        }

        true
    }

    /// Expand free rects when bin grows. Mirrors maxrects-packer expandFreeRects().
    /// `width`/`height` are the target (padded) dimensions; `self.width`/`self.height`
    /// are still the OLD dimensions at this point.
    fn expand_free_rects(&mut self, width: i32, height: i32) {
        let old_w = self.width + self.padding - self.border;
        let old_h = self.height + self.padding - self.border;

        // Extend existing free rects that touch the growth edges
        for free in &mut self.free_rectangles {
            if free.x + free.width >= old_w.min(width) {
                free.width = width - free.x - self.border;
            }
            if free.y + free.height >= old_h.min(height) {
                free.height = height - free.y - self.border;
            }
        }

        // Add the new right strip
        self.free_rectangles.push(Rect::new(
            old_w,
            self.border,
            width - self.width - self.padding,
            height - self.border * 2,
        ));

        // Add the new bottom strip
        self.free_rectangles.push(Rect::new(
            self.border,
            old_h,
            width - self.border * 2,
            height - self.height - self.padding,
        ));

        // Filter out invalid rectangles
        self.free_rectangles.retain(|r| {
            r.width > 0 && r.height > 0 && r.x >= self.border && r.y >= self.border
        });

        self.prune_free_list();
    }

    // === Original MaxRects logic (unchanged) ===

    /// Place a node: split overlapping free rects, prune, add to used.
    pub fn place_rectangle(&mut self, node: &Rect) {
        let mut i = 0;
        while i < self.free_rectangles.len() {
            let free = self.free_rectangles[i];
            if !Self::intersects(&free, node) {
                i += 1;
                continue;
            }
            // Always remove the intersecting free rect. When the node fully covers it,
            // split_free_node returns no splits and the free rect is simply dropped.
            let splits = Self::split_free_node(&free, node);
            self.free_rectangles.remove(i);
            self.free_rectangles.extend(splits);
        }
        self.prune_free_list();
        self.used_rectangles.push(node.clone());
    }

    fn intersects(a: &Rect, b: &Rect) -> bool {
        a.x < b.x + b.width
            && a.x + a.width > b.x
            && a.y < b.y + b.height
            && a.y + a.height > b.y
    }

    fn split_free_node(free_node: &Rect, used_node: &Rect) -> Vec<Rect> {
        if used_node.x >= free_node.x + free_node.width
            || used_node.x + used_node.width <= free_node.x
            || used_node.y >= free_node.y + free_node.height
            || used_node.y + used_node.height <= free_node.y
        {
            return Vec::new();
        }
        let mut result = Vec::new();
        if used_node.x < free_node.x + free_node.width
            && used_node.x + used_node.width > free_node.x
        {
            if used_node.y > free_node.y && used_node.y < free_node.y + free_node.height {
                let mut r = free_node.clone();
                r.height = used_node.y - r.y;
                result.push(r);
            }
            if used_node.y + used_node.height < free_node.y + free_node.height {
                let mut r = free_node.clone();
                r.y = used_node.y + used_node.height;
                r.height = free_node.y + free_node.height - (used_node.y + used_node.height);
                result.push(r);
            }
        }
        if used_node.y < free_node.y + free_node.height
            && used_node.y + used_node.height > free_node.y
        {
            if used_node.x > free_node.x && used_node.x < free_node.x + free_node.width {
                let mut r = free_node.clone();
                r.width = used_node.x - r.x;
                result.push(r);
            }
            if used_node.x + used_node.width < free_node.x + free_node.width {
                let mut r = free_node.clone();
                r.x = used_node.x + used_node.width;
                r.width = free_node.x + free_node.width - (used_node.x + used_node.width);
                result.push(r);
            }
        }
        result
    }

    fn contact_point_score(&self, x: i32, y: i32, w: i32, h: i32) -> i32 {
        let mut score = 0;
        if x == 0 || x + w == self.max_width {
            score += h;
        }
        if y == 0 || y + h == self.max_height {
            score += w;
        }
        for rect in &self.used_rectangles {
            if rect.x == x + w || rect.x + rect.width == x {
                score +=
                    Self::interval_len(rect.y, rect.y + rect.height, y, y + h);
            }
            if rect.y == y + h || rect.y + rect.height == y {
                score +=
                    Self::interval_len(rect.x, rect.x + rect.width, x, x + w);
            }
        }
        score
    }

    fn interval_len(i1s: i32, i1e: i32, i2s: i32, i2e: i32) -> i32 {
        if i1e < i2s || i2e < i1s {
            return 0;
        }
        i1e.min(i2e) - i1s.max(i2s)
    }

    /// Mirrors _pruneFreeList() exactly, including its for-loop quirks: after a splice
    /// the loop index advances, so elements that shift into the removed slot are skipped.
    fn prune_free_list(&mut self) {
        let mut i = 0;
        while i < self.free_rectangles.len() {
            let mut j = i + 1;
            while j < self.free_rectangles.len() {
                // hitTest(i, j): i inside j → remove the earlier rect and restart outer loop
                if self.free_rectangles[j].contains(&self.free_rectangles[i]) {
                    self.free_rectangles.remove(i);
                    break;
                }
                // hitTest(j, i): j inside i → remove j
                if self.free_rectangles[i].contains(&self.free_rectangles[j]) {
                    self.free_rectangles.remove(j);
                }
                j += 1;
            }
            i += 1;
        }
    }

    pub fn occupancy(&self) -> f32 {
        let used: i32 = self.used_rectangles.iter().map(|r| r.width * r.height).sum();
        used as f32 / (self.max_width.max(1) * self.max_height.max(1)) as f32
    }
}

pub(crate) fn next_pow2(v: u32) -> u32 {
    if v == 0 {
        return 1;
    }
    let mut v = v - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bin(
        mw: i32,
        mh: i32,
        pad: i32,
        rot: bool,
        smart: bool,
    ) -> MaxRectsBin {
        let opts = BinOptions {
            smart,
            pot: false,
            square: false,
            allow_rotation: rot,
            border: 0,
            logic: PackMethod::BestShortSideFit,
        };
        MaxRectsBin::new(mw, mh, pad, rot, &opts)
    }

    #[test]
    fn test_place_two_rects() {
        let mut bin = make_bin(256, 256, 0, false, false);
        let r1 = bin.place(100, 100, PackMethod::BestShortSideFit).unwrap();
        let r2 = bin.place(150, 100, PackMethod::BestShortSideFit).unwrap();
        // No overlap
        let a = Rect::new(r1.x, r1.y, r1.width, r1.height);
        let b = Rect::new(r2.x, r2.y, r2.width, r2.height);
        assert!(!overlap(&a, &b));
    }

    #[test]
    fn test_smart_growth() {
        let mut bin = make_bin(256, 256, 0, false, true);
        bin.place(100, 100, PackMethod::BestShortSideFit).unwrap();
        assert!(bin.width > 0);
        assert!(bin.height > 0);
        // With smart mode, bin should be just large enough
        let r2 = bin.place(150, 100, PackMethod::BestShortSideFit).unwrap();
        assert!(r2.x >= 0);
        assert!(r2.y >= 0);
    }

    #[test]
    fn test_fill_width() {
        let mut bin = make_bin(256, 256, 0, false, true);
        let r1 = bin.place(100, 50, PackMethod::FillWidth).unwrap();
        let r2 = bin.place(100, 50, PackMethod::FillWidth).unwrap();
        // FillWidth should place them side-by-side if possible
        let a = Rect::new(r1.x, r1.y, r1.width, r1.height);
        let b = Rect::new(r2.x, r2.y, r2.width, r2.height);
        assert!(!overlap(&a, &b));
    }

    #[test]
    fn test_full_bin() {
        let mut bin = make_bin(128, 128, 0, false, false);
        assert!(bin.place(120, 120, PackMethod::BestShortSideFit).is_some());
        assert!(bin.place(20, 20, PackMethod::BestShortSideFit).is_none());
    }

    #[test]
    fn test_rotation() {
        let mut bin = make_bin(256, 256, 0, true, true);
        let r = bin.place(250, 50, PackMethod::BestShortSideFit).unwrap();
        // Should be placed (possibly rotated)
        assert!(r.x >= 0 && r.y >= 0);
    }

    fn overlap(a: &Rect, b: &Rect) -> bool {
        a.x < b.x + b.width
            && a.x + a.width > b.x
            && a.y < b.y + b.height
            && a.y + a.height > b.y
    }
}
