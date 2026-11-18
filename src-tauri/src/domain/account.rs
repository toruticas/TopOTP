use data_encoding::BASE32_NOPAD;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{AppError, AppResult};

pub const MAX_COUNTER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum OtpKind {
    Totp,
    Hotp,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ts_rs::TS)]
pub enum OtpAlgorithm {
    SHA1,
    SHA256,
    SHA512,
}

pub struct Secret(Zeroizing<Vec<u8>>);

impl Secret {
    pub fn parse(input: &str) -> AppResult<Self> {
        if input.len() > 1024 {
            return Err(AppError::invalid("secret"));
        }
        let mut normalized = Zeroizing::new(String::with_capacity(input.len()));
        for byte in input.bytes() {
            if !byte.is_ascii_whitespace() {
                normalized.push(byte.to_ascii_uppercase() as char);
            }
        }
        let normalized = normalized.trim_end_matches('=');
        let bytes = BASE32_NOPAD
            .decode(normalized.as_bytes())
            .map_err(|_| AppError::invalid("secret"))?;
        if bytes.is_empty() {
            return Err(AppError::invalid("secret"));
        }
        Ok(Self(Zeroizing::new(bytes)))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn encoded(&self) -> Zeroizing<String> {
        Zeroizing::new(BASE32_NOPAD.encode(&self.0))
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.encoded())
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded = Zeroizing::new(String::deserialize(deserializer)?);
        Self::parse(&encoded).map_err(|_| serde::de::Error::custom("invalid OTP secret"))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub schema_version: u8,
    pub id: Uuid,
    #[serde(default)]
    pub provenance: Option<String>,
    pub issuer: String,
    pub account: String,
    pub kind: OtpKind,
    pub algorithm: OtpAlgorithm,
    pub digits: u8,
    pub period: u32,
    pub next_counter: u64,
    pub secret: Secret,
}

#[derive(Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountInput {
    pub issuer: String,
    pub account: String,
    pub kind: OtpKind,
    pub algorithm: OtpAlgorithm,
    pub digits: u8,
    pub period: u32,
    pub next_counter: String,
    #[ts(type = "string | null")]
    pub secret: Option<Zeroizing<String>>,
}

#[derive(Clone, Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub issuer: String,
    pub account: String,
    pub kind: OtpKind,
    pub algorithm: OtpAlgorithm,
    pub digits: u8,
    pub period: u32,
    pub next_counter: Option<String>,
    pub pinned: bool,
}

impl AccountRecord {
    pub fn from_input(input: &AccountInput, previous: Option<&Self>) -> AppResult<Self> {
        let secret = match input.secret.as_deref().filter(|value| !value.is_empty()) {
            Some(value) => Secret::parse(value)?,
            None => previous
                .map(|record| Secret::parse(&record.secret.encoded()))
                .transpose()?
                .ok_or_else(|| AppError::invalid("secret"))?,
        };
        let mut counter = input
            .next_counter
            .parse::<u64>()
            .map_err(|_| AppError::invalid("nextCounter"))?;
        if input.kind == OtpKind::Totp {
            counter = previous.map_or(0, |r| r.next_counter);
        }
        if let Some(previous) = previous
            && counter < previous.next_counter
        {
            return Err(AppError::invalid("nextCounter"));
        }
        let record = Self {
            schema_version: 1,
            id: previous.map_or_else(Uuid::new_v4, |record| record.id),
            provenance: previous.and_then(|r| r.provenance.clone()),
            issuer: input.issuer.trim().to_owned(),
            account: input.account.trim().to_owned(),
            kind: input.kind,
            algorithm: input.algorithm,
            digits: input.digits,
            period: input.period,
            next_counter: counter,
            secret,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.provenance.as_ref().is_some_and(|v| v.len() > 128) {
            return Err(AppError::LimitExceeded);
        }
        if self.schema_version != 1 {
            return Err(AppError::UnsupportedFormat);
        }
        if self.issuer.len() > 1024 || self.account.len() > 1024 {
            return Err(AppError::invalid("account"));
        }
        if !(6..=8).contains(&self.digits) {
            return Err(AppError::invalid("digits"));
        }
        if !(1..=86400).contains(&self.period) {
            return Err(AppError::invalid("period"));
        }
        if self.next_counter > MAX_COUNTER {
            return Err(AppError::invalid("nextCounter"));
        }
        Ok(())
    }

    pub fn summary(&self, pinned: bool) -> AccountSummary {
        AccountSummary {
            id: self.id.to_string(),
            issuer: self.issuer.clone(),
            account: self.account.clone(),
            kind: self.kind,
            algorithm: self.algorithm,
            digits: self.digits,
            period: self.period,
            next_counter: Some(self.next_counter.to_string()),
            pinned,
        }
    }

    pub fn same_identity(&self, other: &Self) -> bool {
        self.provenance
            .as_ref()
            .is_some_and(|p| other.provenance.as_ref() == Some(p))
            || (self.kind == other.kind
                && self.algorithm == other.algorithm
                && self.digits == other.digits
                && (self.kind == OtpKind::Hotp || self.period == other.period)
                && self.secret.bytes() == other.secret.bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_kind_cannot_reset_a_consumed_counter() {
        let previous = crate::compatibility::uri::parse_uri(
            "otpauth://hotp/test?secret=JBSWY3DPEHPK3PXP&counter=8",
        )
        .unwrap();
        let mut input = AccountInput {
            issuer: String::new(),
            account: "test".into(),
            kind: OtpKind::Totp,
            algorithm: OtpAlgorithm::SHA1,
            digits: 6,
            period: 30,
            next_counter: "0".into(),
            secret: None,
        };
        let totp = AccountRecord::from_input(&input, Some(&previous)).unwrap();
        assert_eq!(totp.next_counter, 8);
        input.kind = OtpKind::Hotp;
        assert!(AccountRecord::from_input(&input, Some(&totp)).is_err());
    }

    #[test]
    fn secret_is_redacted_and_invalid_values_rejected() {
        let secret = Secret::parse("JBSW Y3DP EHPK 3PXP").unwrap();
        assert_eq!(format!("{secret:?}"), "[REDACTED]");
        for bad in ["", "INVALID!", "JBSW=Y3DP", "🗝️"] {
            assert!(Secret::parse(bad).is_err());
        }
    }

    #[test]
    fn summary_never_serializes_secret() {
        let record = AccountRecord {
            schema_version: 1,
            id: Uuid::new_v4(),
            provenance: None,
            issuer: "Synthetic".into(),
            account: "RFC".into(),
            kind: OtpKind::Totp,
            algorithm: OtpAlgorithm::SHA1,
            digits: 6,
            period: 30,
            next_counter: 0,
            secret: Secret::parse("JBSWY3DPEHPK3PXP").unwrap(),
        };
        let output = serde_json::to_string(&record.summary(false)).unwrap();
        assert!(!output.contains("secret"));
        assert!(!output.contains("JBSWY3DPEHPK3PXP"));
    }
}
