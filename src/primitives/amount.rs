use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// An `Amount` specifies a fixed-precision quantity supporting up to four digits past
/// the decimal point.
///
/// Values five or more places past the decimal point are considered "dust" and discarded.
///
/// It supports simple mathematical operations and conversions to/from strings.
///
/// ## Libraries which were not used
///
/// `rust_decimal::Decimal` is not suitable because it intrinsically signed and does not
/// provide a method to simply and reliably discard dust.
///
/// `fixed::FixedU64` is not suitable because it is oriented around binary fixed-precision
/// numbers, not decimal fixed-precision numbers. Its smallest value cannot be `0.0001`.
/// This introduces the possibility of rounding errors, which are undesirable.
///
/// `sp_arithmetic::rational::Rational128` is not suitable because it requires building
/// large parts of Substrate, which is enormous.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::Sub,
    derive_more::SubAssign,
)]
pub struct Amount(u64);

impl Amount {
    pub const ZERO: Amount = Amount(0);

    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseAmountError {
    #[error("invalid format")]
    InvalidFormat,
    #[error("out of range: the supplied value cannot fit into the underlying type")]
    OutOfRange,
}

#[derive(Debug, thiserror::Error)]
pub enum AmountFromF64Error {
    #[error("amounts must be finite and a number")]
    NonNormal,
    #[error("amounts must not be negative")]
    Negative,
    #[error(transparent)]
    Fallback(ParseAmountError),
}

static AMOUNT_RE: Lazy<Regex> = Lazy::new(|| {
    // Rules for this regex:
    //
    // - captures only numbers, not mepty strings
    // - any match has at least one digit captured in `pre`
    // - decimal is optional, but if present, must be followed by at least one digit in `post`
    // - dust is discarded at the parse stage
    Regex::new(r"^(?P<pre>\d+)(\.(?P<post>\d{1,4})(?P<dust>\d*))?$")
        .expect("this regular expression will always compile successfully")
});

/// Amounts are represented as a u64 whose value is this many times the true amount.
const AMOUNT_MULTIPLIER: u64 = 10_000;

impl FromStr for Amount {
    type Err = ParseAmountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = AMOUNT_RE
            .captures(s)
            .ok_or(ParseAmountError::InvalidFormat)?;

        let mut value = AMOUNT_MULTIPLIER
            * captures
                .name("pre")
                .expect("any match has at least one digit captured in `pre`")
                .as_str()
                .parse::<u64>()
                .map_err(|_| ParseAmountError::OutOfRange)?;
        if let Some(post_str) = captures.name("post") {
            let post_str = post_str.as_str().trim_end_matches('0');
            let multiplier = 10_u64.pow((4 - post_str.len()) as u32);
            value += multiplier
                * post_str
                    .parse::<u64>()
                    .expect("any set of 1-4 digits should parse successfully");
        }

        Ok(Amount(value))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pre = self.0 / AMOUNT_MULTIPLIER;
        let post = self.0 % AMOUNT_MULTIPLIER;
        if post == 0 {
            write!(f, "{pre}")
        } else {
            write!(f, "{pre}.{post:04}")
        }
    }
}

impl TryFrom<f64> for Amount {
    type Error = AmountFromF64Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_normal() && value != 0.0 {
            return Err(AmountFromF64Error::NonNormal);
        }
        if value < 0.0 {
            return Err(AmountFromF64Error::Negative);
        }

        let parsed_value = (AMOUNT_MULTIPLIER as f64 * value).floor() as u64;
        // `f64` can't represent integers over `(2**53 - 1)` accurately.
        if parsed_value > 9007199254740991 {
            // let's try a safer, slower alternative
            value
                .to_string()
                .parse()
                .map_err(AmountFromF64Error::Fallback)
        } else {
            Ok(Amount(parsed_value))
        }
    }
}

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0 as f64 / AMOUNT_MULTIPLIER as f64)
    }
}

struct AmountVisitor;

impl<'de> serde::de::Visitor<'de> for AmountVisitor {
    type Value = Amount;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a positive number with optional decimal")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value.parse().map_err(serde::de::Error::custom)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value.try_into().map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_f64(AmountVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // potential future tests:
    //
    // - parsing over the entire range (property test)
    // - test designed to trigger `OutOfRange`

    proptest! {
        #[test]
        fn parse_amount_discards_dust(pre in 0u64..=9_999_999_999, post in 1000_u64..=9999, dust in 1_u64..=9999) {
            let expect = (pre * AMOUNT_MULTIPLIER) + post;
            let string = format!("{pre}.{post}{dust}");
            let amount: Amount = string.parse().expect("this generated string is valid");
            prop_assert_eq!(amount.0, expect);
        }

        #[test]
        fn parse_amount_handles_post_properly(pre in 0_u64..=999, post in 1_u64..=9999) {
            let expect = (pre * AMOUNT_MULTIPLIER) + post;
            let string = format!("{pre}.{post:04}");
            let truncated = string.trim_end_matches('0');
            let amount: Amount = truncated.parse().expect("this generated string is valid");
            prop_assert_eq!(amount.0, expect);
        }
    }
}
