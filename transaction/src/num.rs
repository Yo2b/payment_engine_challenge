//! A module providing transaction numeric features.

use std::{fmt, ops, str::FromStr};

use serde::{de, Deserialize, Serialize};

/// The largest precision that could be represented by this decimal type.
pub const MAX_N: u8 = u64::MAX.ilog10() as u8; // 19

/// A decimal handling fixed-precision with up to `N` places past the decimal.
///
/// Safety: `N` is statically checked at compile type and could never exceed `MAX_N`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal<const N: u8>(u64);

impl<const N: u8> Decimal<N> {
    /// The largest value that can be represented by this decimal type.
    pub const MAX: Decimal<N> = Decimal(u64::MAX);
    /// The smallest value that can be represented by this decimal type.
    pub const MIN: Decimal<N> = Decimal(u64::MIN);

    /// The largest integer value that can be represented by this decimal type.
    ///
    /// It is safe to consider that `Self::MAX_UINT < Self::MAX`.
    ///
    /// Caution: `MAX_UINT`.`MAX_FRAC` is typically greater than `MAX` and could not be represented by this decimal type.
    pub const MAX_UINT: u64 = u64::MAX / Self::FRAC;
    /// The largest fractional value that can be represented by this decimal type.
    pub const MAX_FRAC: u64 = Self::FRAC - 1;

    /// The constant denominator internally used to compute fractional values.
    const FRAC: u64 = u64::pow(10, N as u32);

    /// Create a new decimal.
    ///
    /// # Panics
    /// This method panics if the decimal cannot be represented, ie. if `uint > Self::MAX_UINT`.
    fn new(uint: u64, mut frac: u64) -> Self {
        if N == 0 {
            frac = 0;
        } else if frac == Self::FRAC {
            frac /= 10;
        } else if frac > Self::FRAC {
            let n = u64::pow(10, 1 + frac.ilog10() - N as u32);

            frac = (frac as f64 / n as f64).round() as u64;
        };

        assert!(uint <= Self::MAX_UINT);
        assert!(frac <= Self::MAX_FRAC);

        Self(uint * Self::FRAC + frac)
    }

    /// Split this decimal into its integer / fractional parts.
    #[inline]
    fn split(&self) -> (u64, u64) {
        (self.0 / Self::FRAC, self.0 % Self::FRAC)
    }
}

impl<const N: u8> Default for Decimal<N> {
    #[inline]
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl<const N: u8> From<u64> for Decimal<N> {
    #[inline]
    fn from(uint: u64) -> Self {
        Self::new(uint, 0)
    }
}

impl<const N: u8> fmt::Debug for Decimal<N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl<const N: u8> fmt::Display for Decimal<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (uint, frac) = self.split();

        if f.alternate() {
            write!(f, "{uint}.{frac:0>width$}", width = N as usize)
        } else if frac > 0 {
            // TODO: optimize
            let frac = format!("{frac:0>width$}", width = N as usize);
            let frac = frac.trim_end_matches('0');

            write!(f, "{uint}.{frac}")
        } else {
            write!(f, "{uint}")
        }
    }
}

impl<const N: u8> FromStr for Decimal<N> {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (uint, frac) = match s.split_once('.').map(|(u, f)| (u, f.trim_end_matches('0'))) {
            None => (s.parse()?, 0),
            Some((u, "")) => (u.parse()?, 0),
            Some((u, f)) if f.len() < N as usize => (u.parse()?, f.parse::<u64>()? * u64::pow(10, N as u32 - f.len() as u32)),
            Some((u, f)) if f.len() > N as usize && f.starts_with('0') => {
                let f = &f[..N as usize + 1]; // ignore all extra digits except the first one to round up or down
                (u.parse()?, (f.parse::<u64>()? as f64 / 10.0).round() as u64)
            }
            Some((u, f)) => (u.parse()?, f.parse()?),
        };

        Ok(Self::new(uint, frac))
    }
}

impl<const N: u8> ops::Add for Decimal<N> {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl<const N: u8> ops::AddAssign for Decimal<N> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl<const N: u8> ops::Sub for Decimal<N> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl<const N: u8> ops::SubAssign for Decimal<N> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl<const N: u8> Serialize for Decimal<N> {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de, const N: u8> Deserialize<'de> for Decimal<N> {
    #[inline]
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <&str>::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_range() {
        assert_eq!(Decimal::<0>::MAX_FRAC, 0);
        assert_eq!(Decimal::<1>::MAX_FRAC, 9);
        assert_eq!(Decimal::<10>::MAX_FRAC, 9999999999);

        assert_eq!(Decimal::<0>::MAX_UINT, 18446744073709551615);
        assert_eq!(Decimal::<1>::MAX_UINT, 1844674407370955161);
        assert_eq!(Decimal::<10>::MAX_UINT, 1844674407);

        assert_eq!(Decimal::<4>::MIN.to_string(), "0");
        assert_eq!(Decimal::<4>::MAX.to_string(), "1844674407370955.1615");

        let _ = Decimal::<{ MAX_N }>::default();
        // let _ = Decimal::<{MAX_N + 1}>::default(); // cannot compile!
    }

    #[test]
    fn test_decimal_valid_max() {
        let _: Decimal<8> = From::from(Decimal::<8>::MAX_UINT);
    }

    #[test]
    #[should_panic(expected = "assertion failed: uint <= Self::MAX_UINT")]
    fn test_decimal_invalid_max() {
        let _: Decimal<8> = From::from(Decimal::<8>::MAX_UINT + 1); // panics!
    }

    #[test]
    fn test_decimal_from_values() {
        // zero decimal
        assert_eq!(Decimal::<0>::new(0, 0), Decimal(0));
        assert_eq!(Decimal::<0>::new(1, 0), Decimal(1));
        assert_eq!(Decimal::<0>::new(1, 2345), Decimal(1));

        // zero value
        assert_eq!(Decimal::<3>::new(0, 0), Decimal(0));

        // integer value
        assert_eq!(Decimal::<3>::new(5, 0), Decimal(5_000));

        // exact value
        assert_eq!(Decimal::<4>::new(3, 14), Decimal(3_0014));
        assert_eq!(Decimal::<4>::new(3, 1416), Decimal(3_1416));

        // round value
        assert_eq!(Decimal::<4>::new(3, 14159), Decimal(3_1416));
        assert_eq!(Decimal::<3>::new(3, 14159), Decimal(3_142));

        // trailing zeroes
        assert_eq!(Decimal::<3>::new(2, 10), Decimal(2_010));
        assert_eq!(Decimal::<3>::new(2, 1000), Decimal(2_100));
        assert_eq!(Decimal::<3>::new(2, 10000), Decimal(2_100));
    }

    #[test]
    fn test_decimal_from_string() {
        // integer value
        assert_eq!(Decimal::<4>::from_str("30").unwrap(), Decimal(30_0000));
        assert_eq!(Decimal::<4>::from_str("30.").unwrap(), Decimal(30_0000));
        assert_eq!(Decimal::<4>::from_str("30.0").unwrap(), Decimal(30_0000));

        // exact value
        assert_eq!(Decimal::<4>::from_str("3.1416").unwrap(), Decimal(3_1416));

        // round value
        assert_eq!(Decimal::<4>::from_str("3.14159").unwrap(), Decimal(3_1416));

        // missing trailing zeroes
        assert_eq!(Decimal::<4>::from_str("3.14").unwrap(), Decimal(3_1400));
        assert_eq!(Decimal::<4>::from_str("3.014").unwrap(), Decimal(3_0140));

        // exact leading/trailing zeroes
        assert_eq!(Decimal::<4>::from_str("3.1400").unwrap(), Decimal(3_1400));
        assert_eq!(Decimal::<4>::from_str("3.0140").unwrap(), Decimal(3_0140));
        assert_eq!(Decimal::<4>::from_str("3.0014").unwrap(), Decimal(3_0014));

        // extra trailing zeroes
        assert_eq!(Decimal::<4>::from_str("3.14000").unwrap(), Decimal(3_1400));
        assert_eq!(Decimal::<4>::from_str("3.01400").unwrap(), Decimal(3_0140));
        assert_eq!(Decimal::<4>::from_str("3.00140").unwrap(), Decimal(3_0014));

        // extra leading zeroes
        assert_eq!(Decimal::<4>::from_str("3.014159").unwrap(), Decimal(3_0142));
        assert_eq!(Decimal::<4>::from_str("3.0014159").unwrap(), Decimal(3_0014));
        assert_eq!(Decimal::<4>::from_str("3.00014159").unwrap(), Decimal(3_0001));
        assert_eq!(Decimal::<4>::from_str("3.000014159").unwrap(), Decimal(3_0000));
        assert_eq!(Decimal::<4>::from_str("3.0000014159").unwrap(), Decimal(3_0000));

        // round value limits
        assert_eq!(Decimal::<4>::from_str("1.00024999").unwrap(), Decimal(1_0002));
        assert_eq!(Decimal::<4>::from_str("1.00025001").unwrap(), Decimal(1_0003));
    }

    #[test]
    fn test_decimal_to_string() {
        assert_eq!(format!("{}", Decimal::<4>::from(30)), "30");
        assert_eq!(format!("{:#}", Decimal::<4>::from(30)), "30.0000");

        assert_eq!(format!("{}", Decimal::<4>::new(3, 1400)), "3.14");
        assert_eq!(format!("{:#}", Decimal::<4>::new(3, 1400)), "3.1400");

        assert_eq!(Decimal::<4>::new(3, 14).to_string(), "3.0014");
        assert_eq!(Decimal::<4>::new(3, 1416).to_string(), "3.1416");
        assert_eq!(Decimal::<4>::new(3, 14159).to_string(), "3.1416");
    }

    #[test]
    #[allow(clippy::zero_prefixed_literal)]
    fn test_decimal_valid_ops() {
        let mut a = Decimal::<4>::new(3, 14159);
        let mut b = Decimal::<4>::new(1, 41421);

        assert_eq!(a + b, Decimal(4_5558));
        assert_eq!(a - b, Decimal(1_7274));

        let e = Decimal::<4>::new(2, 71828);

        a -= e;
        b += e;

        assert_eq!(a, Decimal(0_4233));
        assert_eq!(b, Decimal(4_1325));
    }

    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn test_decimal_invalid_ops() {
        let a = Decimal::<4>::new(3, 14159);
        let b = Decimal::<4>::new(1, 41421);

        let _ = b - a; // panics!
    }
}
