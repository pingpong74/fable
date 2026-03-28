use crate::ComponentId;
use std::hash::{BuildHasher, Hasher};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

// this can store our components.. super fast ig ?
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct BitSet {
    id: u128,
}

impl BitSet {
    pub(crate) fn from_component_ids(ids: &[&'static ComponentId]) -> BitSet {
        let mut res: u128 = 0;

        for id in ids {
            res |= 1 << id.get_id();
        }

        return BitSet { id: res };
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.id.count_ones() as usize
    }

    pub(crate) fn iter(self) -> BitSetIterator {
        BitSetIterator { bits: self.id }
    }
}

pub(crate) struct BitSetIterator {
    bits: u128,
}

impl Iterator for BitSetIterator {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            return None;
        }
        // Find the index of the rightmost set bit
        let bit_index = self.bits.trailing_zeros() as usize;
        // Clear the rightmost set bit
        self.bits &= self.bits - 1;
        Some(bit_index)
    }
}

impl BitAnd for BitSet {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        BitSet { id: self.id & rhs.id }
    }
}

impl BitAndAssign for BitSet {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.id &= rhs.id;
    }
}

impl BitOr for BitSet {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        BitSet { id: self.id | rhs.id }
    }
}

impl BitOrAssign for BitSet {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.id |= rhs.id;
    }
}

impl BitXor for BitSet {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self {
        BitSet { id: self.id ^ rhs.id }
    }
}

impl BitXorAssign for BitSet {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.id ^= rhs.id;
    }
}

impl Not for BitSet {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        BitSet { id: !self.id }
    }
}

pub(crate) struct BitSetHasher {
    hash: u64,
}

impl Hasher for BitSetHasher {
    fn finish(&self) -> u64 {
        return self.hash;
    }

    fn write(&mut self, _bytes: &[u8]) {
        unreachable!("We will use specialized write methods");
    }

    // Specialized for your [u128; 4] bitset
    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.hash = (i as u64) ^ ((i >> 64) as u64);
    }
}

#[derive(Default)]
pub(crate) struct BuildBitSetHasher;

impl BuildHasher for BuildBitSetHasher {
    type Hasher = BitSetHasher;
    fn build_hasher(&self) -> Self::Hasher {
        BitSetHasher { hash: 0 }
    }
}
