//! Logic for matching frame families.
//!
//! Since there are only 3 relevant family strings ("native", "javascript", and "other"),
//! plus the wildcard "all" that matches any family, we can concisely represent them using one byte.

/// A bit field representing a list of allowed families.
///
/// * `0b001` represents `"other"`
/// * `0b010` represents `"native"`
/// * `0b100` represents `"javascript"`
/// * `u8::MAX` represents `"all"`
#[derive(Debug, Clone, Copy)]
pub struct Families(u8);

const BITFIELD_OTHER: u8 = 0b001;
const BITFIELD_NATIVE: u8 = 0b010;
const BITFIELD_JAVASCRIPT: u8 = 0b100;
const BITFIELD_ALL: u8 = u8::MAX;

impl Families {
    /// Creates a [`Families`] structure from a comma-separated list of families.
    pub fn new(families: &str) -> Self {
        let mut bitfield = 0;
        for family in families.split(',') {
            bitfield |= match family {
                "other" => BITFIELD_OTHER,
                "native" => BITFIELD_NATIVE,
                "javascript" => BITFIELD_JAVASCRIPT,
                "all" => BITFIELD_ALL,
                _ => 0,
            };
        }
        Self(bitfield)
    }

    /// Checks whether `self` and `other` have at least one family in common, where
    /// `all` counts as all families.
    pub fn matches(&self, other: Families) -> bool {
        (self.0 & other.0) > 0
    }
}

impl Default for Families {
    fn default() -> Self {
        Self(BITFIELD_OTHER)
    }
}
