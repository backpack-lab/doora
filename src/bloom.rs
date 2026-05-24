#![allow(dead_code)]

//! Space-efficient probabilistic Bloom filter utilities used to pre-filter
//! files by trigrams before expensive parsing or regex checks.

/// Number of bits in the per-file Bloom filter.
///
/// Chosen to balance memory usage and false positive rate for typical
/// repositories scanned by this tool.
pub const BLOOM_BITS: usize = 4096;

/// Number of bytes backing the Bloom filter (`BLOOM_BITS / 8`).
pub const BLOOM_BYTES: usize = 512;

/// Number of hash functions used by the Bloom filter implementation.
pub const NUM_HASH_FUNCTIONS: usize = 2;

/// A small, fixed-size Bloom filter for trigram membership tests.
///
/// `BloomFilter` supports inserting 3-byte trigrams and testing for probable
/// membership. It provides zero false negatives for inserted trigrams and may
/// yield false positives for non-inserted trigrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BloomFilter {
    bits: [u8; BLOOM_BYTES],
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl BloomFilter {
    /// Create a new, empty `BloomFilter` with all bits cleared.
    pub fn new() -> Self {
        BloomFilter { bits: [0u8; BLOOM_BYTES] }
    }

    /// Insert a 3-byte `trigram` into the filter.
    ///
    /// This operation sets two bits derived from the trigram and is
    /// idempotent.
    pub fn insert(&mut self, trigram: &[u8; 3]) {
        let (i1, i2) = bit_indices(trigram);
        let b1 = i1 / 8;
        let o1 = (i1 % 8) as u8;
        self.bits[b1] |= 1 << o1;
        let b2 = i2 / 8;
        let o2 = (i2 % 8) as u8;
        self.bits[b2] |= 1 << o2;
    }

    /// Test whether `trigram` is probably present in the filter.
    ///
    /// Guarantees zero false negatives for trigrams that were previously
    /// inserted. False positives are possible and increase with the number
    /// of inserted trigrams; see `false_positive_estimate` for an estimate.
    pub fn probably_contains(&self, trigram: &[u8; 3]) -> bool {
        let (i1, i2) = bit_indices(trigram);
        let b1 = i1 / 8;
        let o1 = (i1 % 8) as u8;
        if ((self.bits[b1] >> o1) & 1) != 1 {
            return false;
        }
        let b2 = i2 / 8;
        let o2 = (i2 % 8) as u8;
        ((self.bits[b2] >> o2) & 1) == 1
    }

    /// Insert a batch of trigrams into the filter.
    ///
    /// This is a convenience wrapper around `insert` and performs bulk
    /// insertion without additional transactional semantics.
    pub fn insert_trigrams(&mut self, trigrams: &[[u8; 3]]) {
        for t in trigrams {
            self.insert(t);
        }
    }

    /// Return `true` when the filter probably contains all `trigrams`.
    ///
    /// This calls `probably_contains` for each trigram and returns `false`
    /// immediately on the first missing trigram.
    pub fn probably_contains_all(&self, trigrams: &[[u8; 3]]) -> bool {
        for t in trigrams {
            if !self.probably_contains(t) {
                return false;
            }
        }
        true
    }

    /// Serialize the filter to a fixed-size byte array.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; BLOOM_BYTES] {
        self.bits
    }

    /// Construct a `BloomFilter` from a fixed-size byte array previously
    /// produced by `to_bytes`.
    pub fn from_bytes(bytes: [u8; BLOOM_BYTES]) -> Self {
        BloomFilter { bits: bytes }
    }

    /// Estimate the false positive probability for the current filter state.
    ///
    /// The estimate is derived from the fraction of set bits and the number
    /// of hash functions; it is an approximation useful for diagnostics.
    #[must_use]
    pub fn false_positive_estimate(&self) -> f64 {
        let set_bits = self.bit_count();
        let fraction = (set_bits as f64) / (BLOOM_BITS as f64);
        fraction.powi(NUM_HASH_FUNCTIONS as i32)
    }

    /// Count the number of bits currently set in the filter.
    #[must_use]
    pub fn bit_count(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }
}

fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 2166136261u32;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619u32);
    }
    hash
}

fn fnv1a_32_alt(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x5E18FF4E;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619u32);
    }
    hash
}

fn bit_indices(trigram: &[u8; 3]) -> (usize, usize) {
    let h1 = fnv1a_32(trigram) as usize % BLOOM_BITS;
    let h2 = fnv1a_32_alt(trigram) as usize % BLOOM_BITS;
    (h1, h2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigram::{extract_query_trigrams, extract_unique_trigrams};

    #[test]
    fn test_new_filter_all_zeros() {
        let f = BloomFilter::new();
        assert_eq!(f.to_bytes(), [0u8; BLOOM_BYTES]);
    }

    #[test]
    fn test_insert_sets_bits() {
        let mut f = BloomFilter::new();
        f.insert(&[b'f', b'n', b' ']);
        let count = f.bit_count();
        assert!(count > 0);
        assert!(count <= 2);
    }

    #[test]
    fn test_probably_contains_after_insert() {
        let mut f = BloomFilter::new();
        let t = [b't', b'e', b's'];
        f.insert(&t);
        assert!(f.probably_contains(&t));
    }

    #[test]
    fn test_probably_contains_false_for_empty() {
        let f = BloomFilter::new();
        assert!(!f.probably_contains(&[b'a', b'b', b'c']));
    }

    #[test]
    fn test_zero_false_negatives_for_inserted_trigrams() {
        let mut f = BloomFilter::new();
        let source = "fn authenticate(user: &str) -> bool { true }";
        let trigrams = extract_unique_trigrams(source);
        f.insert_trigrams(&trigrams);
        for t in trigrams.iter() {
            assert!(f.probably_contains(t));
        }
    }

    #[test]
    fn test_insert_trigrams_batch() {
        let mut f1 = BloomFilter::new();
        let mut f2 = BloomFilter::new();
        let trigrams = vec![[b'a', b'b', b'c'], [b'd', b'e', b'f']];
        f1.insert_trigrams(&trigrams);
        f2.insert(&[b'a', b'b', b'c']);
        f2.insert(&[b'd', b'e', b'f']);
        assert_eq!(f1, f2);
    }

    #[test]
    fn test_probably_contains_all_true_when_all_inserted() {
        let mut f = BloomFilter::new();
        let a = [b'a', b'b', b'c'];
        let b = [b'd', b'e', b'f'];
        let c = [b'g', b'h', b'i'];
        f.insert(&a);
        f.insert(&b);
        f.insert(&c);
        assert!(f.probably_contains_all(&[a, b, c]));
    }

    #[test]
    fn test_probably_contains_all_false_when_one_missing() {
        let mut f = BloomFilter::new();
        let a = [b'a', b'b', b'c'];
        let b = [b'd', b'e', b'f'];
        let c = [0xFFu8, 0xFEu8, 0xFDu8];
        f.insert(&a);
        f.insert(&b);
        assert!(!f.probably_contains_all(&[a, b, c]));
    }

    #[test]
    fn test_to_bytes_from_bytes_roundtrip() {
        let mut f = BloomFilter::new();
        f.insert(&[b'a', b'b', b'c']);
        let bytes = f.to_bytes();
        let g = BloomFilter::from_bytes(bytes);
        assert_eq!(f, g);
    }

    #[test]
    fn test_bit_count_increases_with_inserts() {
        let mut f = BloomFilter::new();
        assert_eq!(f.bit_count(), 0);
        f.insert(&[b'a', b'b', b'c']);
        let c1 = f.bit_count();
        assert!(c1 >= 1 && c1 <= 2);
        f.insert(&[b'd', b'e', b'f']);
        let c2 = f.bit_count();
        assert!(c2 <= 4);
    }

    #[test]
    fn test_false_positive_estimate_zero_for_empty() {
        let f = BloomFilter::new();
        assert_eq!(f.false_positive_estimate(), 0.0);
    }

    #[test]
    fn test_false_positive_estimate_increases_with_inserts() {
        let mut f = BloomFilter::new();
        assert_eq!(f.false_positive_estimate(), 0.0);
        for i in 0..500u16 {
            let t = [(i % 256) as u8, ((i + 1) % 256) as u8, ((i + 2) % 256) as u8];
            f.insert(&t);
        }
        let est = f.false_positive_estimate();
        assert!(est > 0.0 && est < 1.0);
    }

    #[test]
    fn test_hash_functions_are_deterministic() {
        let t = [b'a', b'b', b'c'];
        let i1 = bit_indices(&t);
        let i2 = bit_indices(&t);
        assert_eq!(i1, i2);
    }

    #[test]
    fn test_hash_functions_are_independent() {
        let samples = vec![
            [b'a', b'b', b'c'],
            [b'f', b'n', b' '],
            [b'g', b'o', b'o'],
            [b'h', b'e', b'y'],
            [b'r', b'u', b's'],
            [b't', b'e', b's'],
            [b'u', b'n', b'i'],
            [b'v', b'a', b'r'],
            [b'x', b'y', b'z'],
            [b'k', b'l', b'm'],
        ];
        let mut distinct = 0usize;
        for s in samples {
            let (a, b) = bit_indices(&s);
            if a != b {
                distinct += 1;
            }
        }
        assert!(distinct >= 8);
    }

    #[test]
    fn test_full_pipeline_with_trigram_module() {
        let source = "fn authenticate(user: &str) -> bool { true }";
        let trigrams = extract_unique_trigrams(source);
        let mut f = BloomFilter::new();
        f.insert_trigrams(&trigrams);
        let query = extract_query_trigrams("authenticate");
        assert!(f.probably_contains_all(&query));
    }

    #[test]
    fn test_bloom_filter_size_is_512_bytes() {
        assert_eq!(size_of::<BloomFilter>(), BLOOM_BYTES);
    }

    #[test]
    fn test_bloom_filter_is_clone() {
        let f = BloomFilter::new();
        let _ = f.clone();
    }
}
