use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{
  de::Error as DeserializeError, ser::Error as SerializeError, Deserialize, Deserializer,
  Serialize, Serializer,
};
use serde_json::Number as JsonNumber;

use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct SafeDecimal(#[schemars(with = "f64")] u64);

impl SafeDecimal {
  const MAX: u32 = 1_000_000_000;
  const DECIMALS: u32 = 6;
  const SCALE: u32 = 10u32.pow(Self::DECIMALS);
  const MAX_VAL: u64 = Self::MAX as u64 * Self::SCALE as u64 - 1;

  pub fn new(integral: u32, fractional: u32) -> Result<Self> {
    if integral >= Self::MAX || fractional >= Self::SCALE {
      return Err(Error::Overflow {});
    }
    Ok(Self(integral as u64 * Self::SCALE as u64 + fractional as u64))
  }

  pub fn integral(&self) -> u32 {
    (self.0 / Self::SCALE as u64) as u32
  }

  pub fn fractional(&self) -> u32 {
    (self.0 % Self::SCALE as u64) as u32
  }
}

impl Add for SafeDecimal {
  type Output = Result<SafeDecimal>;

  fn add(self, rhs: Self) -> Self::Output {
    Ok(Self(self.0.checked_add(rhs.0).ok_or(Error::Overflow {})?))
  }
}

impl Sub for SafeDecimal {
  type Output = Result<SafeDecimal>;

  fn sub(self, rhs: Self) -> Self::Output {
    Ok(Self(self.0.checked_sub(rhs.0).ok_or(Error::Overflow {})?))
  }
}

impl Mul for SafeDecimal {
  type Output = Result<SafeDecimal>;

  fn mul(self, rhs: Self) -> Self::Output {
    let res = self.0 as u128 * rhs.0 as u128 / Self::SCALE as u128;
    if res > Self::MAX_VAL as u128 {
      return Err(Error::Overflow {});
    }
    Ok(Self(res as u64))
  }
}

impl Div for SafeDecimal {
  type Output = Result<SafeDecimal>;

  fn div(self, rhs: Self) -> Self::Output {
    let res = self.0 as u128 * Self::SCALE as u128 / rhs.0 as u128;
    if res > Self::MAX_VAL as u128 {
      return Err(Error::Overflow {});
    }
    Ok(Self(res as u64))
  }
}

impl FromStr for SafeDecimal {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self> {
    let mut parts = s.split('.');
    let integral: u32 = parts.next().ok_or(Error::UnexpectedFormat {})?.parse()?;
    let maybe_fractional: Option<u32> =
      parts.next().map(|s| format!("{:0<6}", s.trim_end_matches("0")).parse()).transpose()?;
    if parts.next().is_some() {
      return Err(Error::UnexpectedFormat {});
    }
    match maybe_fractional {
      Some(fractional) => Self::new(integral, fractional),
      None => Self::new(integral, 0),
    }
  }
}

impl<'de> Deserialize<'de> for SafeDecimal {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where D: Deserializer<'de> {
    String::deserialize(deserializer)?.parse().map_err(D::Error::custom)
  }
}

impl Serialize for SafeDecimal {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where S: Serializer {
    self.to_string().parse::<JsonNumber>().map_err(S::Error::custom)?.serialize(serializer)
  }
}

impl Display for SafeDecimal {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    let integral = self.integral();
    let fractional = self.fractional();
    if fractional == 0 {
      write!(f, "{}", integral)
    } else {
      let mut frac_str = format!("{:06}", fractional);
      frac_str = frac_str.trim_end_matches('0').to_string();
      write!(f, "{}.{}", integral, frac_str)
    }
  }
}

impl TryFrom<u32> for SafeDecimal {
  type Error = Error;

  fn try_from(value: u32) -> Result<Self> {
    Self::new(value, 0)
  }
}

impl TryFrom<u64> for SafeDecimal {
  type Error = Error;

  fn try_from(value: u64) -> Result<Self> {
    if value > u32::MAX as u64 {
      return Err(Error::Overflow {});
    }
    Self::new(value as u32, 0)
  }
}

impl TryFrom<u128> for SafeDecimal {
  type Error = Error;

  fn try_from(value: u128) -> Result<Self> {
    if value > u32::MAX as u128 {
      return Err(Error::Overflow {});
    }
    Self::new(value as u32, 0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_valid() {
    let decimal = SafeDecimal::new(123, 456789).unwrap();
    assert_eq!(decimal.integral(), 123);
    assert_eq!(decimal.fractional(), 456789);
  }

  #[test]
  fn test_new_overflow() {
    assert!(matches!(SafeDecimal::new(1_000_000_000, 0), Err(Error::Overflow {})));
    assert!(matches!(SafeDecimal::new(0, 1_000_000), Err(Error::Overflow {})));
  }

  #[test]
  fn test_add() {
    let a = SafeDecimal::new(1, 500000).unwrap();
    let b = SafeDecimal::new(2, 700000).unwrap();
    let result = (a + b).unwrap();
    assert_eq!(result.integral(), 4);
    assert_eq!(result.fractional(), 200000);
  }

  #[test]
  fn test_sub() {
    let a = SafeDecimal::new(5, 300000).unwrap();
    let b = SafeDecimal::new(2, 100000).unwrap();
    let result = (a - b).unwrap();
    assert_eq!(result.integral(), 3);
    assert_eq!(result.fractional(), 200000);
  }

  #[test]
  fn test_mul() {
    let a = SafeDecimal::new(2, 500000).unwrap();
    let b = SafeDecimal::new(3, 0).unwrap();
    let result = (a * b).unwrap();
    assert_eq!(result.integral(), 7);
    assert_eq!(result.fractional(), 500000);
  }

  #[test]
  fn test_div() {
    let a = SafeDecimal::new(10, 0).unwrap();
    let b = SafeDecimal::new(2, 0).unwrap();
    let result = (a / b).unwrap();
    assert_eq!(result.integral(), 5);
    assert_eq!(result.fractional(), 0);
  }

  #[test]
  fn test_display() {
    let decimal = SafeDecimal::new(123, 456789).unwrap();
    assert_eq!(format!("{}", decimal), "123.456789");
  }

  #[test]
  fn test_from_str_valid_integer() {
    let result = SafeDecimal::from_str("123").unwrap();
    assert_eq!(result.to_string(), "123");
  }

  #[test]
  fn test_from_str_valid_decimal() {
    let result = SafeDecimal::from_str("123.45").unwrap();
    assert_eq!(result.to_string(), "123.45");
  }

  #[test]
  fn test_from_str_invalid_format() {
    assert!(SafeDecimal::from_str("123.45.67").is_err());
  }

  #[test]
  fn test_from_str_empty_string() {
    assert!(SafeDecimal::from_str("").is_err());
  }

  #[test]
  fn test_deserialize_valid() {
    let json = serde_json::json!("123.45");
    let result: SafeDecimal = serde_json::from_value(json).unwrap();
    assert_eq!(result.to_string(), "123.45");
  }

  #[test]
  fn test_deserialize_invalid() {
    let json = serde_json::json!("invalid");
    let result: Result<SafeDecimal, _> = serde_json::from_value(json);
    assert!(result.is_err());
  }

  #[test]
  fn test_serialize() {
    let decimal = SafeDecimal::from_str("123.45").unwrap();
    let json = serde_json::to_value(decimal).unwrap();
    assert_eq!(json, serde_json::json!(123.45));
  }

  #[test]
  fn test_try_from_u32() {
    let decimal: SafeDecimal = 42u32.try_into().unwrap();
    assert_eq!(decimal.integral(), 42);
    assert_eq!(decimal.fractional(), 0);
  }

  #[test]
  fn test_try_from_u64() {
    let decimal: SafeDecimal = 123456789u64.try_into().unwrap();
    assert_eq!(decimal.integral(), 123456789);
    assert_eq!(decimal.fractional(), 0);
  }

  #[test]
  fn test_try_from_u128_overflow() {
    let result: Result<SafeDecimal, Error> = (u128::MAX).try_into();
    assert!(matches!(result, Err(Error::Overflow {})));
  }
}
