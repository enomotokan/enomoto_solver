//! A small, fast, non-cryptographic hasher for the presolve passes' own
//! lookup tables.
//!
//! Several presolve passes group rows or columns by an exact "normalized
//! coefficient signature" (`Vec<(usize, u64)>`, one entry per nonzero) in a
//! `HashMap`/`HashSet`, once per pass and — for the inequality dedup — once
//! per presolve *round*. The standard library's default `SipHash` is built
//! to resist adversarial keys, which costs several times more per hashed
//! word than this crate needs for keys it builds itself from its own
//! matrix; those hashes were a visible part of presolve on the larger
//! Netlib instances.
//!
//! This is the multiply-rotate scheme of `rustc`'s own `FxHasher`. It is
//! only used by tables that are **looked up**, never iterated in hash
//! order (or iterated and then sorted), so swapping the hasher cannot
//! change any pass's result — only how fast it gets there.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(buf));
        }
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;
pub type FxHashSet<K> = HashSet<K, FxBuildHasher>;
