use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use zeroize::Zeroizing;

use super::{
    AppResult,
    account::{AccountRecord, OtpAlgorithm},
};

pub fn hotp(secret: &[u8], counter: u64, algorithm: OtpAlgorithm, digits: u8) -> String {
    let message = counter.to_be_bytes();
    let digest = match algorithm {
        OtpAlgorithm::SHA1 => {
            let mut hmac =
                Hmac::<Sha1>::new_from_slice(secret).expect("HMAC accepts any key length");
            hmac.update(&message);
            hmac.finalize().into_bytes().to_vec()
        }
        OtpAlgorithm::SHA256 => {
            let mut hmac =
                Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
            hmac.update(&message);
            hmac.finalize().into_bytes().to_vec()
        }
        OtpAlgorithm::SHA512 => {
            let mut hmac =
                Hmac::<Sha512>::new_from_slice(secret).expect("HMAC accepts any key length");
            hmac.update(&message);
            hmac.finalize().into_bytes().to_vec()
        }
    };
    let digest = Zeroizing::new(digest);
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let binary = u32::from_be_bytes(digest[offset..offset + 4].try_into().unwrap()) & 0x7fff_ffff;
    format!(
        "{:0width$}",
        binary % 10_u32.pow(digits as u32),
        width = digits as usize
    )
}

pub fn totp(record: &AccountRecord, unix_seconds: u64) -> AppResult<(String, u64)> {
    record.validate()?;
    let period = record.period as u64;
    Ok((
        hotp(
            record.secret.bytes(),
            unix_seconds / period,
            record.algorithm,
            record.digits,
        ),
        (unix_seconds / period + 1) * period,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_4226_all_counters() {
        let expected = [
            "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583",
            "399871", "520489",
        ];
        for (counter, expected) in expected.iter().enumerate() {
            assert_eq!(
                hotp(
                    b"12345678901234567890",
                    counter as u64,
                    OtpAlgorithm::SHA1,
                    6
                ),
                *expected
            );
        }
    }

    #[test]
    fn rfc_6238_all_algorithms_and_times() {
        let cases = [
            (59, ["94287082", "46119246", "90693936"]),
            (1111111109, ["07081804", "68084774", "25091201"]),
            (1111111111, ["14050471", "67062674", "99943326"]),
            (1234567890, ["89005924", "91819424", "93441116"]),
            (2000000000, ["69279037", "90698825", "38618901"]),
            (20000000000, ["65353130", "77737706", "47863826"]),
        ];
        let secrets: [&[u8]; 3] = [
            b"12345678901234567890",
            b"12345678901234567890123456789012",
            b"1234567890123456789012345678901234567890123456789012345678901234",
        ];
        let algorithms = [
            OtpAlgorithm::SHA1,
            OtpAlgorithm::SHA256,
            OtpAlgorithm::SHA512,
        ];
        for (time, expected) in cases {
            for i in 0..3 {
                assert_eq!(hotp(secrets[i], time / 30, algorithms[i], 8), expected[i]);
            }
        }
    }

    #[test]
    fn custom_period_and_clock_boundaries() {
        let r = crate::compatibility::uri::parse_uri(
            "otpauth://totp/test?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&period=45&digits=8",
        )
        .unwrap();
        let (before, until) = totp(&r, 44).unwrap();
        assert_eq!(until, 45);
        assert_eq!(before, hotp(r.secret.bytes(), 0, r.algorithm, 8));
        assert_ne!(before, totp(&r, 45).unwrap().0);
        assert_eq!(totp(&r, 44).unwrap().0, before);
    }
    #[test]
    fn independent_node_hmac_variants() {
        let values: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/hotp-reference.json"))
                .unwrap();
        for v in values.as_array().unwrap() {
            let algorithm = serde_json::from_value(v["algorithm"].clone()).unwrap();
            assert_eq!(
                hotp(
                    b"12345678901234567890",
                    v["counter"].as_u64().unwrap(),
                    algorithm,
                    6
                ),
                v["code"].as_str().unwrap()
            );
        }
    }
}
