use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, Copy)]
pub struct OrderedF32(pub f32);

#[derive(Debug, Clone, Copy)]
pub struct OrderedF64(pub f64);

impl OrderedF32 {
    fn canonical_bits(&self) -> u32 {
        if self.0.is_nan() {
            0x7fc00000
        } else if self.0 == 0.0 {
            0x00000000
        } else {
            self.0.to_bits()
        }
    }
}

impl OrderedF64 {
    fn canonical_bits(&self) -> u64 {
        if self.0.is_nan() {
            0x7ff8000000000000
        } else if self.0 == 0.0 {
            0x0000000000000000
        } else {
            self.0.to_bits()
        }
    }
}

impl Eq for OrderedF32 {}
impl PartialEq for OrderedF32 {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_bits() == other.canonical_bits()
    }
}

impl Hash for OrderedF32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical_bits().hash(state);
    }
}

impl PartialOrd for OrderedF32 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF32 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical_bits().cmp(&other.canonical_bits())
    }
}

impl Eq for OrderedF64 {}
impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_bits() == other.canonical_bits()
    }
}

impl Hash for OrderedF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical_bits().hash(state);
    }
}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical_bits().cmp(&other.canonical_bits())
    }
}
