#[derive(Debug, Clone, Copy)]
pub struct Families(u8);

const BITFIELD_OTHER: u8 = 0b001;
const BITFIELD_NATIVE: u8 = 0b010;
const BITFIELD_JAVASCRIPT: u8 = 0b100;
const BITFIELD_ALL: u8 = u8::MAX;

impl Families {
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

    pub fn matches(&self, other: Families) -> bool {
        (self.0 & other.0) > 0
    }
}

impl Default for Families {
    fn default() -> Self {
        Self(BITFIELD_OTHER)
    }
}
