//! The table file (F1, SPEC 5.4): a header, the value array in [`StateSpace`] order, and a SHA-256 checksum.
//!
//! Layout, all integers little-endian:
//!
//! | offset | size | field |
//! | --- | --- | --- |
//! | 0 | 8 | magic `YATZYTBL` |
//! | 8 | 4 | format version (1) |
//! | 12 | 4 | header length in bytes: the offset of the values, a multiple of 64 |
//! | 16 | 8 | state count |
//! | 24 | 1 | categories |
//! | 25 | 1 | armed values (1 or 2) |
//! | 26 | 2 | upper values (bonus threshold + 1, or 1) |
//! | 28 | 1 | precision: bytes per value, 4 (f32) or 8 (f64) |
//! | 29 | 1 | objective: 0 = maximize the expected final score |
//! | 30 | 2 | reserved, 0 |
//! | 32 | 32 | SHA-256 of the variant's [`Variant::canonical_text`] |
//! | 64 | 2 + n | variant id: length, then UTF-8 bytes |
//! | ... | 2 + n | solver version: length, then UTF-8 bytes |
//! | ... | ... | zero padding to the header length |
//! | header length | count x precision | the values, IEEE 754 |
//! | end - 32 | 32 | SHA-256 of everything before it |
//!
//! The values start at a 64-byte boundary, so the array can be memory-mapped.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::solver::{StateSpace, TurnModel};
use crate::variant::Variant;

pub const MAGIC: &[u8; 8] = b"YATZYTBL";
pub const FORMAT_VERSION: u32 = 1;
/// The version of the solver that wrote a table.
pub const SOLVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the table's values optimize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// Maximize the expected final score.
    ExpectedScore = 0,
}

/// Bytes per stored value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    F32 = 4,
    F64 = 8,
}

/// A table error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableError(pub String);

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "table: {}", self.0)
    }
}

impl std::error::Error for TableError {}

/// A solved table: V for every state of a variant.
#[derive(Clone, Debug)]
pub struct Table {
    variant: Variant,
    precision: Precision,
    solver_version: String,
    /// The values, widened to f64 (exact for f32 tables).
    values: Vec<f64>,
}

/// SHA-256 of a variant's canonical text.
pub fn variant_hash(v: &Variant) -> [u8; 32] {
    Sha256::digest(v.canonical_text().as_bytes()).into()
}

/// Lowercase hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Table {
    /// Solves a variant (in f64) and keeps the values at the given precision.
    pub fn build(v: &Variant, precision: Precision) -> Table {
        let model = TurnModel::new(v);
        let values: Vec<f64> = model.solve();
        Table::from_values(v, precision, values)
    }

    /// A table from solved values, rounded to the given precision.
    pub fn from_values(v: &Variant, precision: Precision, mut values: Vec<f64>) -> Table {
        assert_eq!(values.len(), StateSpace::of(v).len());
        if precision == Precision::F32 {
            for x in &mut values {
                *x = f64::from(*x as f32);
            }
        }
        Table { variant: v.clone(), precision, solver_version: SOLVER_VERSION.into(), values }
    }

    pub fn variant(&self) -> &Variant {
        &self.variant
    }

    pub fn precision(&self) -> Precision {
        self.precision
    }

    pub fn solver_version(&self) -> &str {
        &self.solver_version
    }

    /// The values in [`StateSpace`] order.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The file bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let space = StateSpace::of(&self.variant);
        let mut h = Vec::with_capacity(128);
        h.extend_from_slice(MAGIC);
        h.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes()); // header length, patched below
        h.extend_from_slice(&(space.len() as u64).to_le_bytes());
        h.push(space.categories as u8);
        h.push(space.armed_values as u8);
        h.extend_from_slice(&(space.upper_values as u16).to_le_bytes());
        h.push(self.precision as u8);
        h.push(Objective::ExpectedScore as u8);
        h.extend_from_slice(&[0, 0]);
        h.extend_from_slice(&variant_hash(&self.variant));
        for s in [self.variant.id(), self.solver_version.as_str()] {
            h.extend_from_slice(&(s.len() as u16).to_le_bytes());
            h.extend_from_slice(s.as_bytes());
        }
        h.resize(h.len().div_ceil(64) * 64, 0);
        let header_len = h.len() as u32;
        h[12..16].copy_from_slice(&header_len.to_le_bytes());
        let mut out = h;
        out.reserve(self.values.len() * self.precision as usize + 32);
        for &x in &self.values {
            match self.precision {
                Precision::F32 => out.extend_from_slice(&(x as f32).to_le_bytes()),
                Precision::F64 => out.extend_from_slice(&x.to_le_bytes()),
            }
        }
        let sum = Sha256::digest(&out);
        out.extend_from_slice(&sum);
        out
    }

    /// Reads a table and checks it: magic, version, checksum, and that it was built for exactly this variant
    /// definition.
    pub fn from_bytes(bytes: &[u8], v: &Variant) -> Result<Table, TableError> {
        let err = |s: &str| Err(TableError(s.to_string()));
        if bytes.len() < 64 + 32 || &bytes[..8] != MAGIC {
            return err("not a yatzy-solver table");
        }
        let (body, sum) = bytes.split_at(bytes.len() - 32);
        if Sha256::digest(body).as_slice() != sum {
            return err("checksum mismatch: the file is damaged");
        }
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        if u32_at(8) != FORMAT_VERSION {
            return Err(TableError(format!("format version {} is not supported", u32_at(8))));
        }
        let header_len = u32_at(12) as usize;
        let count = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;
        let precision = match bytes[28] {
            4 => Precision::F32,
            8 => Precision::F64,
            p => return Err(TableError(format!("unknown precision {p}"))),
        };
        if bytes[29] != Objective::ExpectedScore as u8 {
            return Err(TableError(format!("unknown objective {}", bytes[29])));
        }
        if bytes[32..64] != variant_hash(v) {
            return Err(TableError(format!("the table was not built for variant {} as defined here", v.id())));
        }
        let space = StateSpace::of(v);
        if count != space.len()
            || usize::from(bytes[24]) != space.categories
            || usize::from(bytes[25]) != space.armed_values
            || usize::from(u16::from_le_bytes([bytes[26], bytes[27]])) != space.upper_values
        {
            return err("state space does not match the variant");
        }
        let mut o = 64;
        let mut read_str = || -> Result<String, TableError> {
            let n = usize::from(u16::from_le_bytes([bytes[o], bytes[o + 1]]));
            let s = std::str::from_utf8(&bytes[o + 2..o + 2 + n]).map_err(|_| TableError("bad string".into()))?;
            o += 2 + n;
            Ok(s.to_string())
        };
        let id = read_str()?;
        let solver_version = read_str()?;
        if id != v.id() {
            return Err(TableError(format!("table is for {id}, not {}", v.id())));
        }
        let p = precision as usize;
        if body.len() != header_len + count * p {
            return err("file length does not match the header");
        }
        let data = &body[header_len..];
        let values = match precision {
            Precision::F32 => {
                data.chunks_exact(4).map(|c| f64::from(f32::from_le_bytes(c.try_into().unwrap()))).collect()
            }
            Precision::F64 => data.chunks_exact(8).map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect(),
        };
        Ok(Table { variant: v.clone(), precision, solver_version, values })
    }

    /// SHA-256 of the value array alone, independent of the header (and so of the solver version).
    pub fn values_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        for &x in &self.values {
            match self.precision {
                Precision::F32 => h.update((x as f32).to_le_bytes()),
                Precision::F64 => h.update(x.to_le_bytes()),
            }
        }
        h.finalize().into()
    }
}

#[cfg(all(test, feature = "verify"))]
mod tests {
    use super::*;
    use crate::verify::reduced_variants;

    #[test]
    fn round_trip_and_damage_detection() {
        let v = &reduced_variants()[0];
        for p in [Precision::F32, Precision::F64] {
            let t = Table::build(v, p);
            let bytes = t.to_bytes();
            let header_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            assert_eq!(header_len % 64, 0);
            let back = Table::from_bytes(&bytes, v).unwrap();
            assert_eq!(back.values(), t.values());
            assert_eq!(back.values_hash(), t.values_hash());
            let mut bad = bytes.clone();
            bad[header_len + 3] ^= 1;
            assert!(Table::from_bytes(&bad, v).unwrap_err().0.contains("checksum"));
            assert!(Table::from_bytes(&bytes, &reduced_variants()[1]).is_err());
        }
    }
}
