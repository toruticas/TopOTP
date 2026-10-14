use std::collections::HashMap;
use url::Url;
use zeroize::Zeroizing;

use crate::domain::{
    AppError, AppResult,
    account::{AccountInput, AccountRecord, OtpAlgorithm, OtpKind},
};

pub fn parse_uri(uri: &str) -> AppResult<AccountRecord> {
    if uri.len() > 8192 {
        return Err(AppError::LimitExceeded);
    }
    let uri = Url::parse(uri).map_err(|_| AppError::invalid("uri"))?;
    if uri.scheme() != "otpauth" {
        return Err(AppError::UnsupportedFormat);
    }
    if !uri.username().is_empty()
        || uri.password().is_some()
        || uri.port().is_some()
        || uri.fragment().is_some()
    {
        return Err(AppError::invalid("uri"));
    }
    let kind = match uri.host_str() {
        Some("totp") => OtpKind::Totp,
        Some("hotp") => OtpKind::Hotp,
        _ => return Err(AppError::UnsupportedFormat),
    };
    let mut params = HashMap::new();
    for (key, value) in uri.query_pairs() {
        if ![
            "secret",
            "issuer",
            "algorithm",
            "digits",
            "period",
            "counter",
        ]
        .contains(&key.as_ref())
        {
            return Err(AppError::UnsupportedFormat);
        }
        if params
            .insert(key.into_owned(), Zeroizing::new(value.into_owned()))
            .is_some()
        {
            return Err(AppError::invalid("uri"));
        }
    }
    let secret = params
        .remove("secret")
        .ok_or_else(|| AppError::invalid("secret"))?;
    let label = percent_encoding::percent_decode_str(uri.path().trim_start_matches('/'))
        .decode_utf8()
        .map_err(|_| AppError::invalid("uri"))?;
    let (label_issuer, account) = label.split_once(':').unwrap_or(("", &label));
    let issuer = params
        .remove("issuer")
        .map(|value| value.to_string())
        .unwrap_or_else(|| label_issuer.to_owned());
    if !label_issuer.is_empty() && label_issuer != issuer {
        return Err(AppError::invalid("issuer"));
    }
    let algorithm = match params
        .get("algorithm")
        .map(|value| value.to_ascii_uppercase())
        .as_deref()
    {
        None | Some("SHA1") => OtpAlgorithm::SHA1,
        Some("SHA256") => OtpAlgorithm::SHA256,
        Some("SHA512") => OtpAlgorithm::SHA512,
        _ => return Err(AppError::UnsupportedFormat),
    };
    let digits = params
        .get("digits")
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| AppError::invalid("digits"))?
        .unwrap_or(6);
    let period = params
        .get("period")
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| AppError::invalid("period"))?
        .unwrap_or(30);
    let counter = match (kind, params.remove("counter")) {
        (OtpKind::Hotp, None) => return Err(AppError::invalid("nextCounter")),
        (_, Some(value)) => value.to_string(),
        _ => "0".to_owned(),
    };
    AccountRecord::from_input(
        &AccountInput {
            issuer,
            account: account.to_owned(),
            kind,
            algorithm,
            digits,
            period,
            next_counter: counter,
            secret: Some(secret),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_required_counter() {
        let record =
            parse_uri("otpauth://totp/Synthetic:alice?secret=JBSWY3DPEHPK3PXP&issuer=Synthetic")
                .unwrap();
        assert_eq!(record.issuer, "Synthetic");
        assert_eq!(record.account, "alice");
        assert_eq!(record.digits, 6);
        assert_eq!(record.period, 30);
        assert!(parse_uri("otpauth://hotp/alice?secret=JBSWY3DPEHPK3PXP").is_err());
        assert!(parse_uri("otpauth://totp/alice?secret=X&secret=Y").is_err());
    }

    #[test]
    fn labels_and_unsupported_parameters() {
        let r = parse_uri("otpauth://totp/A%26B%2Btest?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(r.account, "A&B+test");
        assert!(parse_uri("otpauth://totp/A:b?secret=JBSWY3DPEHPK3PXP&issuer=C").is_err());
        assert!(parse_uri("otpauth://totp/a?secret=JBSWY3DPEHPK3PXP&algorithm=MD5").is_err());
        assert!(
            parse_uri("otpauth://totp/a?secret=JBSWY3DPEHPK3PXP&image=https://example.com")
                .is_err()
        );
    }
}
