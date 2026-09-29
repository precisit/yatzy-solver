//! The number type of expected values.
//!
//! The solver is generic over [`Value`] so the same code runs in `f64` (production tables) and, with the
//! `verify` feature, in exact rational arithmetic (the brute-force cross-check on reduced games).

/// A number the solver can compute expectations with.
pub trait Value: Clone + PartialOrd + Send + Sync + std::fmt::Debug {
    fn zero() -> Self;
    fn from_points(p: u16) -> Self;
    fn add_assign(&mut self, other: &Self);
    /// Divides by the number of faces (6).
    fn div_faces(&mut self);
    /// Multiplies by a small integer.
    fn mul_int(&mut self, k: u64);
    /// Divides by a small integer.
    fn div_int(&mut self, k: u64);
    /// Replaces `self` with `other` when `other` is larger.
    fn max_assign(&mut self, other: &Self) {
        if *other > *self {
            *self = other.clone();
        }
    }
    /// The nearest `f64`.
    fn to_f64(&self) -> f64;
}

impl Value for f64 {
    #[inline]
    fn zero() -> Self {
        0.0
    }
    #[inline]
    fn from_points(p: u16) -> Self {
        f64::from(p)
    }
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        *self += *other;
    }
    #[inline]
    fn div_faces(&mut self) {
        *self /= 6.0;
    }
    #[inline]
    fn mul_int(&mut self, k: u64) {
        *self *= k as f64;
    }
    #[inline]
    fn div_int(&mut self, k: u64) {
        *self /= k as f64;
    }
    #[inline]
    fn max_assign(&mut self, other: &Self) {
        if *other > *self {
            *self = *other;
        }
    }
    #[inline]
    fn to_f64(&self) -> f64 {
        *self
    }
}

#[cfg(feature = "verify")]
mod exact {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{ToPrimitive, Zero};

    impl super::Value for BigRational {
        fn zero() -> Self {
            <BigRational as Zero>::zero()
        }
        fn from_points(p: u16) -> Self {
            BigRational::from_integer(BigInt::from(p))
        }
        fn add_assign(&mut self, other: &Self) {
            *self += other;
        }
        fn div_faces(&mut self) {
            *self /= BigInt::from(6);
        }
        fn mul_int(&mut self, k: u64) {
            *self *= BigInt::from(k);
        }
        fn div_int(&mut self, k: u64) {
            *self /= BigInt::from(k);
        }
        fn to_f64(&self) -> f64 {
            ToPrimitive::to_f64(self).unwrap_or(f64::NAN)
        }
    }
}
