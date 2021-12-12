//! Sign JWTs using various signature algorithms.
//!
//! See the tests for how to use a specific signing algorithm.

use crate::error::Result;

/// A signature which can be represented by bytes.
pub trait Signature: AsRef<[u8]> + private::Private {}

impl private::Private for Vec<u8> {}
impl Signature for Vec<u8> {}

impl private::Private for &[u8] {}
impl Signature for &[u8] {}

impl private::Private for String {}
impl Signature for String {}

impl private::Private for &str {}
impl Signature for &str {}

/// A type which can sign a byte buffer.
///
/// In some cases, the trait is directly implemented on a signing key type which
/// can directly generate a signature.
///
/// In other cases, a new type composed of multiple fields may be needed because
/// the signing key's sign method may require more parameters (e.g. a random
/// number generator).
pub trait Signer: private::Private {
    type Signature: Signature;

    /// Returns a signature from a byte buffer.
    fn sign(&self, bytes: &[u8]) -> Result<Self::Signature>;
}

impl<T> Signer for &T
where
    T: Signer,
{
    type Signature = T::Signature;

    fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
        T::sign(self, bytes)
    }
}

mod private {
    pub trait Private {}

    impl<T> Private for &T where T: Private {}
}

#[cfg(feature = "p256")]
mod p256 {
    use crate::error::Result;

    impl super::Signature for p256::ecdsa::Signature {}
    impl super::private::Private for p256::ecdsa::Signature {}
    impl super::private::Private for p256::ecdsa::SigningKey {}

    impl super::Signer for p256::ecdsa::SigningKey {
        type Signature = p256::ecdsa::Signature;

        fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
            Ok(p256::ecdsa::signature::Signer::sign(self, bytes))
        }
    }

    #[cfg(test)]
    mod test {
        #[test]
        fn test_rust_crypto_p256() {
            const HEADER: &str = "{\"alg\":\"ES256\",\"typ\":\"JWT\"}";

            let rng = rand::thread_rng();
            crate::encode_and_sign(
                HEADER,
                crate::tests::jwt_claims_str(),
                &::p256::ecdsa::SigningKey::random(rng),
            );

            // assert_eq!("", signer.encode_and_sign_json(HEADER, CLAIMS).unwrap());
        }
    }
}

#[cfg(feature = "rsa")]
pub mod rsa {
    use std::marker::PhantomData;

    use crate::{
        algorithm::{Algorithm, Rs256},
        error::Result,
    };
    use rsa::{Hash, PaddingScheme};

    #[derive(Debug)]
    pub struct RsaPrivateKeySigner<A>
    where
        A: Algorithm,
    {
        key: rsa::RsaPrivateKey,
        alg: core::marker::PhantomData<A>,
    }

    impl<A> super::private::Private for RsaPrivateKeySigner<A> where A: Algorithm {}

    macro_rules! rsa_impl {
        ($alg:ty, $padding:expr) => {
            impl super::Signer for RsaPrivateKeySigner<$alg> {
                type Signature = Vec<u8>;

                fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
                    rsa::RsaPrivateKey::sign(&self.key, $padding, bytes).map_err(|_| todo!())
                }
            }
        };
    }

    rsa_impl!(Rs256, {
        PaddingScheme::PKCS1v15Sign {
            hash: Some(Hash::SHA2_256),
        }
    });

    impl RsaPrivateKeySigner<Rs256> {
        pub fn with_rs256(key: rsa::RsaPrivateKey) -> Self {
            Self {
                key,
                alg: PhantomData::default(),
            }
        }
    }
}

#[cfg(feature = "ring")]
pub mod ring {
    use crate::{
        algorithm::{Algorithm, Rs256},
        error::{Error, Result},
    };
    use ring::rand::SecureRandom;

    impl super::Signature for ::ring::signature::Signature {}
    impl super::private::Private for ::ring::signature::Signature {}
    impl super::Signature for ::ring::hmac::Tag {}
    impl super::private::Private for ::ring::hmac::Tag {}

    #[derive(Debug)]
    pub struct EcdsaKeyPairSigner<R>
    where
        R: SecureRandom,
    {
        key_pair: ::ring::signature::EcdsaKeyPair,
        secure_random: R,
    }

    impl<R> super::private::Private for EcdsaKeyPairSigner<R> where R: SecureRandom {}

    impl<R> EcdsaKeyPairSigner<R>
    where
        R: SecureRandom,
    {
        /// Signs header and claims parts with an ECDSA key.
        ///
        /// ```
        /// # use min_jwt::Error;
        /// #
        /// # fn try_main() -> Result<(), Error> {
        /// use min_jwt::sign::ring::EcdsaKeyPairSigner;
        /// use ring::{rand::SystemRandom};
        ///
        /// let sys_rand = SystemRandom::new();
        ///
        /// let header = String::from("{\"alg\":\"ES256\",\"typ\":\"JWT\"}");
        /// let claims = String::from("{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"admin\":true,\"iat\":1516239022}");
        ///
        /// // Normally the key's bytes are read from a file or another data store
        /// // and should not be randomly generated on every invocation
        /// let pkcs8_bytes = ::ring::signature::EcdsaKeyPair::generate_pkcs8(
        ///   &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        ///   &sys_rand
        /// )?;
        /// let key_pair = ::ring::signature::EcdsaKeyPair::from_pkcs8(
        ///   &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        ///   pkcs8_bytes.as_ref()
        /// )?;
        ///
        /// let signing_key = EcdsaKeyPairSigner::with_key_pair_and_random(key_pair, sys_rand);
        ///
        /// /* the header and claims could be serialized by Serde */
        /// /* in the end, the serialized JSON should be referenced as either &str or &[u8] */
        ///
        /// let jwt = min_jwt::encode_and_sign(&header, &claims, &signing_key)?;
        ///
        /// #   Ok(())
        /// # }
        /// # fn main() {
        /// #   try_main().unwrap();
        /// # }
        /// ```
        pub fn with_key_pair_and_random(
            key_pair: ::ring::signature::EcdsaKeyPair,
            secure_random: R,
        ) -> EcdsaKeyPairSigner<R> {
            Self {
                key_pair,
                secure_random,
            }
        }
    }

    impl<R> super::Signer for EcdsaKeyPairSigner<R>
    where
        R: SecureRandom,
    {
        type Signature = ring::signature::Signature;

        fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
            self.key_pair
                .sign(&self.secure_random, bytes)
                .map_err(|_| todo!())
        }
    }

    impl super::private::Private for ::ring::hmac::Key {}

    impl super::Signer for ::ring::hmac::Key {
        type Signature = ::ring::hmac::Tag;

        fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
            Ok(::ring::hmac::sign(self, bytes))
        }
    }

    #[derive(Debug)]
    pub struct RsaKeyPairSigner<R, A>
    where
        R: SecureRandom,
        A: Algorithm,
    {
        key_pair: ::ring::signature::RsaKeyPair,
        secure_random: R,
        alg: std::marker::PhantomData<A>,
    }

    impl<R, A> super::private::Private for RsaKeyPairSigner<R, A>
    where
        R: SecureRandom,
        A: Algorithm,
    {
    }

    macro_rules! rsa_impl {
        ($alg:ty, $alg_str:expr, $ring_alg:expr) => {
            impl<R> super::Signer for RsaKeyPairSigner<R, $alg>
            where
                R: SecureRandom,
            {
                type Signature = Vec<u8>;

                fn sign(&self, bytes: &[u8]) -> Result<Self::Signature> {
                    let mut signature = vec![0; self.key_pair.public_modulus_len()];
                    self.key_pair
                        .sign(&$ring_alg, &self.secure_random, bytes, &mut signature)
                        .map_err(|_| Error::invalid_signature())?;
                    Ok(signature)
                }
            }
        };
    }

    rsa_impl!(Rs256, "RS256", ring::signature::RSA_PKCS1_SHA256);

    impl<R> RsaKeyPairSigner<R, Rs256>
    where
        R: SecureRandom,
    {
        /// Signs header and claims parts with an RSA key.
        pub fn with_rs256(
            key_pair: ::ring::signature::RsaKeyPair,
            secure_random: R,
        ) -> RsaKeyPairSigner<R, Rs256> {
            Self {
                key_pair,
                secure_random,
                alg: std::marker::PhantomData::<Rs256>::default(),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::EcdsaKeyPairSigner;

        const HEADER: &str = "{\"alg\":\"ES256\",\"typ\":\"JWT\"}";

        #[test]
        fn test_ring_ecdsa_key_pair() {
            let secure_random = ::ring::rand::SystemRandom::new();
            let key_pair = ::ring::signature::EcdsaKeyPair::from_pkcs8(
                &::ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                ::ring::signature::EcdsaKeyPair::generate_pkcs8(
                    &::ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                    &secure_random,
                )
                .unwrap()
                .as_ref(),
            )
            .unwrap();

            let key_pair_with_rand =
                EcdsaKeyPairSigner::with_key_pair_and_random(key_pair, secure_random);
            crate::encode_and_sign(HEADER, crate::tests::jwt_claims_str(), &key_pair_with_rand);
            // assert_eq!("", signer.encode_and_sign_json(HEADER, CLAIMS).unwrap());
        }
    }
}

#[cfg(feature = "web_crypto")]
pub mod web_crypto {
    use js_sys::Uint8Array;
    use web_sys::{CryptoKey, SubtleCrypto};

    use crate::{
        error::Error,
        keys::jwk::{Jwk, USAGE_SIGN},
        web_crypto::WebCryptoAlgorithm,
        Algorithm,
    };

    /// A key used to sign JWTs.
    #[derive(Debug)]
    pub struct Signer<'a> {
        subtle_crypto: &'a SubtleCrypto,
        algorithm: Algorithm,
        crypto_key: CryptoKey,
    }

    impl<'a> Signer<'a> {
        /// Imports a JWK via the `SubtleCrypto` API.
        pub async fn with_jwk<'b>(
            subtle_crypto: &'a SubtleCrypto,
            jwk: &Jwk,
        ) -> Result<Signer<'a>, Error> {
            if let Some(usage) = jwk.r#use.as_deref() {
                if usage != USAGE_SIGN {
                    return Err(Error::key_rejected());
                }
            }

            let algorithm = jwk.algorithm().map_err(|_| Error::key_rejected())?;
            let crypto_key = crate::web_crypto::import_jwk(
                subtle_crypto,
                jwk,
                algorithm,
                crate::web_crypto::KeyUsage::Sign,
            )
            .await?;
            Ok(Signer {
                subtle_crypto,
                crypto_key,
                algorithm,
            })
        }

        /// Returns the algorithm of the underlying key.
        pub fn algorithm(&self) -> Algorithm {
            self.algorithm
        }

        /// Serializes the types to JSON, base64 encodes the JSON, constructs the signing input, signs the data, and then
        /// returns the JWT.
        ///
        /// # Errors
        ///
        /// The function may return an error variant because the key pair is invalid.
        #[cfg(all(feature = "serde", feature = "serde_json"))]
        #[inline]
        pub async fn encode_and_sign<H, C>(&self, header: H, claims: C) -> Result<String, Error>
        where
            H: crate::Header + serde::Serialize,
            C: crate::Claims + serde::Serialize,
        {
            let header = serde_json::to_vec(&header).unwrap();
            let claims = serde_json::to_vec(&claims).unwrap();
            self.encode_and_sign_json(header, claims).await
        }

        /// Base64 encodes the JSON, constructs the signing input, signs the data, and then
        /// returns the JWT.
        ///
        /// # Errors
        ///
        /// The function may return an error variant because the key pair is invalid.
        #[inline]
        pub async fn encode_and_sign_json<H, C>(
            &self,
            header: H,
            claims: C,
        ) -> Result<String, Error>
        where
            H: AsRef<[u8]>,
            C: AsRef<[u8]>,
        {
            let encoded_header = base64::encode_config(header, base64::URL_SAFE_NO_PAD);
            let encoded_claims = base64::encode_config(claims, base64::URL_SAFE_NO_PAD);
            let data_to_sign = [encoded_header.as_ref(), encoded_claims.as_ref()].join(".");

            let signed_data_promise = self
                .subtle_crypto
                .sign_with_object_and_u8_array(
                    &self.algorithm.sign_algorithm(),
                    &self.crypto_key,
                    &mut data_to_sign.clone().into_bytes(),
                )
                .map_err(|_| Error::key_rejected())?;
            let signed_data_array_buffer =
                wasm_bindgen_futures::JsFuture::from(signed_data_promise)
                    .await
                    .map_err(|_| Error::key_rejected())?;
            let signature = base64::encode_config(
                &Uint8Array::new(&signed_data_array_buffer).to_vec(),
                base64::URL_SAFE_NO_PAD,
            );

            let signature = base64::encode_config(&signature, base64::URL_SAFE_NO_PAD);

            Ok([data_to_sign, signature].join("."))
        }
    }
}