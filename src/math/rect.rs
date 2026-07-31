/// Rect represents a rectangle with position (x, y) and size (width, height).
/// Mirrors math/Rect.js from free-tex-packer-core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Returns true if `other` is fully contained within `self`.
    /// Mirrors Rect.hitTest(a, b) — "a contains b?"
    pub fn contains(&self, other: &Rect) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.x + self.width >= other.x + other.width
            && self.y + self.height >= other.y + other.height
    }
}
