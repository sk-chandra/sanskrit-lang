//! A minimal arbitrary-precision integer, implemented from scratch (no external
//! crates). Sign-magnitude, base 2³², little-endian limbs, with a canonical
//! zero (`negative = false`, empty `mag`) so that derived equality is correct.
//!
//! This is deliberately simple rather than fast: enough to make Sūtra's integer
//! arithmetic exact. The common small-integer path stays on `i64` in the engine;
//! `BigInt` is only used once a value no longer fits.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigInt {
    negative: bool,
    mag: Vec<u32>, // little-endian, no trailing zero limbs
}

impl BigInt {
    pub fn zero() -> BigInt {
        BigInt { negative: false, mag: vec![] }
    }

    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    fn normalized(negative: bool, mut mag: Vec<u32>) -> BigInt {
        while mag.last() == Some(&0) {
            mag.pop();
        }
        BigInt { negative: !mag.is_empty() && negative, mag }
    }

    pub fn from_i64(n: i64) -> BigInt {
        let neg = n < 0;
        let m = (n as i128).unsigned_abs();
        Self::from_u128(neg, m)
    }

    fn from_u128(negative: bool, mut m: u128) -> BigInt {
        let mut mag = Vec::new();
        while m > 0 {
            mag.push((m & 0xFFFF_FFFF) as u32);
            m >>= 32;
        }
        Self::normalized(negative, mag)
    }

    /// Convert back to `i64` when it fits.
    pub fn to_i64(&self) -> Option<i64> {
        if self.mag.len() > 2 {
            return None;
        }
        let mut v: u128 = 0;
        for (i, limb) in self.mag.iter().enumerate() {
            v |= (*limb as u128) << (32 * i);
        }
        if self.negative {
            if v <= (i64::MAX as u128) + 1 {
                Some((v as i128).wrapping_neg() as i64)
            } else {
                None
            }
        } else if v <= i64::MAX as u128 {
            Some(v as i64)
        } else {
            None
        }
    }

    pub fn to_f64(&self) -> f64 {
        let mut v = 0.0_f64;
        for limb in self.mag.iter().rev() {
            v = v * 4294967296.0 + (*limb as f64);
        }
        if self.negative {
            -v
        } else {
            v
        }
    }

    pub fn parse_decimal(s: &str) -> Option<BigInt> {
        let (negative, digits) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut mag: Vec<u32> = vec![];
        for ch in digits.bytes() {
            mul_small(&mut mag, 10);
            add_small(&mut mag, (ch - b'0') as u32);
        }
        Some(Self::normalized(negative, mag))
    }

    pub fn to_decimal_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut limbs = self.mag.clone();
        let mut chunks: Vec<u32> = vec![];
        while !limbs.is_empty() {
            let rem = divmod_small(&mut limbs, 1_000_000_000);
            chunks.push(rem);
        }
        let mut out = String::new();
        if self.negative {
            out.push('-');
        }
        // Most-significant chunk without padding, the rest zero-padded to 9.
        out.push_str(&chunks.last().unwrap().to_string());
        for chunk in chunks.iter().rev().skip(1) {
            out.push_str(&format!("{:09}", chunk));
        }
        out
    }

    pub fn neg(&self) -> BigInt {
        Self::normalized(!self.negative, self.mag.clone())
    }

    pub fn add(&self, other: &BigInt) -> BigInt {
        if self.negative == other.negative {
            Self::normalized(self.negative, add_mag(&self.mag, &other.mag))
        } else {
            match cmp_mag(&self.mag, &other.mag) {
                Ordering::Equal => BigInt::zero(),
                Ordering::Greater => Self::normalized(self.negative, sub_mag(&self.mag, &other.mag)),
                Ordering::Less => Self::normalized(other.negative, sub_mag(&other.mag, &self.mag)),
            }
        }
    }

    pub fn sub(&self, other: &BigInt) -> BigInt {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &BigInt) -> BigInt {
        Self::normalized(self.negative != other.negative, mul_mag(&self.mag, &other.mag))
    }

    /// Truncating division and remainder, matching `i64` semantics: the
    /// quotient truncates toward zero and the remainder takes the dividend's
    /// sign. Returns `None` on division by zero.
    pub fn div_rem(&self, other: &BigInt) -> Option<(BigInt, BigInt)> {
        if other.is_zero() {
            return None;
        }
        let (q_mag, r_mag) = divmod_mag(&self.mag, &other.mag);
        let q = Self::normalized(self.negative != other.negative, q_mag);
        let r = Self::normalized(self.negative, r_mag);
        Some((q, r))
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_mag(&self.mag, &other.mag),
            (true, true) => cmp_mag(&other.mag, &self.mag),
        }
    }
}

// ---- magnitude helpers (operate on little-endian Vec<u32>, trimmed) ----

fn trim(mut m: Vec<u32>) -> Vec<u32> {
    while m.last() == Some(&0) {
        m.pop();
    }
    m
}

fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry: u64 = 0;
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0) as u64;
        let y = *b.get(i).unwrap_or(&0) as u64;
        let s = x + y + carry;
        out.push((s & 0xFFFF_FFFF) as u32);
        carry = s >> 32;
    }
    if carry > 0 {
        out.push(carry as u32);
    }
    trim(out)
}

/// Requires a >= b.
fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow: i64 = 0;
    for i in 0..a.len() {
        let x = a[i] as i64;
        let y = *b.get(i).unwrap_or(&0) as i64;
        let mut d = x - y - borrow;
        if d < 0 {
            d += 1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(d as u32);
    }
    trim(out)
}

fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0u32; a.len() + b.len()];
    for (i, &ai) in a.iter().enumerate() {
        let mut carry: u64 = 0;
        for (j, &bj) in b.iter().enumerate() {
            let idx = i + j;
            let cur = out[idx] as u64 + ai as u64 * bj as u64 + carry;
            out[idx] = (cur & 0xFFFF_FFFF) as u32;
            carry = cur >> 32;
        }
        out[i + b.len()] += carry as u32;
    }
    trim(out)
}

/// `m = m * x + 0`; small multiply in place.
fn mul_small(m: &mut Vec<u32>, x: u32) {
    let mut carry: u64 = 0;
    for limb in m.iter_mut() {
        let cur = *limb as u64 * x as u64 + carry;
        *limb = (cur & 0xFFFF_FFFF) as u32;
        carry = cur >> 32;
    }
    while carry > 0 {
        m.push((carry & 0xFFFF_FFFF) as u32);
        carry >>= 32;
    }
}

fn add_small(m: &mut Vec<u32>, x: u32) {
    let mut carry = x as u64;
    let mut i = 0;
    while carry > 0 {
        if i == m.len() {
            m.push(0);
        }
        let cur = m[i] as u64 + carry;
        m[i] = (cur & 0xFFFF_FFFF) as u32;
        carry = cur >> 32;
        i += 1;
    }
}

/// `m = m / divisor` in place, returning the remainder.
fn divmod_small(m: &mut Vec<u32>, divisor: u32) -> u32 {
    let mut rem: u64 = 0;
    for limb in m.iter_mut().rev() {
        let cur = (rem << 32) | *limb as u64;
        *limb = (cur / divisor as u64) as u32;
        rem = cur % divisor as u64;
    }
    while m.last() == Some(&0) {
        m.pop();
    }
    rem as u32
}

fn bit_length(m: &[u32]) -> usize {
    match m.last() {
        None => 0,
        Some(top) => (m.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
    }
}

fn get_bit(m: &[u32], i: usize) -> bool {
    (m[i / 32] >> (i % 32)) & 1 == 1
}

fn set_bit(m: &mut [u32], i: usize) {
    m[i / 32] |= 1 << (i % 32);
}

/// Shift a magnitude left by one bit (multiply by 2), in place.
fn shl1(m: &mut Vec<u32>) {
    let mut carry: u32 = 0;
    for limb in m.iter_mut() {
        let new_carry = *limb >> 31;
        *limb = (*limb << 1) | carry;
        carry = new_carry;
    }
    if carry > 0 {
        m.push(carry);
    }
}

/// Schoolbook binary long division of magnitudes; returns (quotient, remainder).
fn divmod_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if cmp_mag(a, b) == Ordering::Less {
        return (vec![], a.to_vec());
    }
    let nbits = bit_length(a);
    let mut q = vec![0u32; a.len()];
    let mut r: Vec<u32> = vec![];
    for i in (0..nbits).rev() {
        shl1(&mut r);
        if get_bit(a, i) {
            if r.is_empty() {
                r.push(1);
            } else {
                r[0] |= 1;
            }
        }
        if cmp_mag(&r, b) != Ordering::Less {
            r = sub_mag(&r, b);
            set_bit(&mut q, i);
        }
    }
    (trim(q), trim(r))
}
