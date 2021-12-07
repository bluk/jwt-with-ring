//! PKCS8 Keys.
//!
//! Keys in PEM/DER format.

use crate::Algorithm;

/// A key used to sign, verify, or encrypt data.
#[derive(Clone, Debug)]
pub struct Pkcs8Key<'a> {
    pub(crate) algorithm: Algorithm,
    pub(crate) data: &'a [u8],
}

impl<'a> Pkcs8Key<'a> {
    pub fn with_rs256_key(data: &'a [u8]) -> Self {
        Self {
            algorithm: Algorithm::Rs256,
            data,
        }
    }

    pub fn with_es256_key(data: &'a [u8]) -> Self {
        Self {
            algorithm: Algorithm::Es256,
            data,
        }
    }
}

/// A key used to sign, verify, or encrypt data with the key ID.
#[derive(Clone, Debug)]
pub struct Pkcs8KeyWithId<'a, 'b> {
    pub(crate) key: Pkcs8Key<'a>,
    pub(crate) kid: &'b str,
}

impl<'a, 'b> Pkcs8KeyWithId<'a, 'b> {
    pub fn with_key_and_id(key: Pkcs8Key<'a>, kid: &'b str) -> Self {
        Self { key, kid }
    }
}
