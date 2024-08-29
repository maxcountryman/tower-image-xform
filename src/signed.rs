use base64::{engine::general_purpose::URL_SAFE, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use url::Url;

use crate::{
    transformation_params::{Height, TransformationParams, Width},
    Key,
};

/// Verifier of signatures.
#[derive(Debug, Clone)]
pub struct Verifier {
    key: Key,
}

impl Verifier {
    /// Create a new [`Verifier`] with the provided [`Key`].
    pub const fn new(key: Key) -> Self {
        Self { key }
    }

    /// Verify a given signature and value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tower_image_xform::{Key, Verifier};
    ///
    /// # /*
    /// let key = { /* a cryptographically random key >= 64 bytes */ };
    /// # */
    /// # let key: &Vec<u8> = &(0..64).collect();
    ///
    /// let key = Key::from(key);
    /// let verifier = Verifier::new(key);
    ///
    /// # /*
    /// let sig = { /* a signature produced by signing a value */ };
    /// let val = { /* a signed value, e.g. transform parameters and URL */ };
    /// # */
    /// # let sig = "ZkGOa8OrigopaLapeyNwVkREmYauORdo9OYh3-2rvQY=";
    /// # let val = "foobar";
    ///
    /// assert!(verifier.verify(sig, val));
    /// ```
    pub fn verify(&self, signature: &str, value: &str) -> bool {
        let Ok(digest) = URL_SAFE.decode(signature) else {
            tracing::warn!("could not Base64 decode signature");
            return false;
        };

        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_slice())
            .expect("HMAC can take key of any size");
        mac.update(value.as_bytes());
        mac.verify_slice(&digest).is_ok()
    }
}

/// Signed URL.
#[derive(Debug)]
pub struct SignedUrl {
    base: Url,
    key: Key,
    params: TransformationParams,
    target: Url,
}

impl SignedUrl {
    const fn new(key: Key, base: Url, target: Url, params: TransformationParams) -> Self {
        Self {
            base,
            key,
            params,
            target,
        }
    }

    fn sign(&self, data: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_slice())
            .expect("HMAC can take key of any size");
        mac.update(data);
        URL_SAFE.encode(mac.finalize().into_bytes())
    }

    /// Generates a signed URL.
    ///
    /// The signature is based on the parameters and encoded URL.
    pub fn generate_signed_url(&self) -> Result<Url, url::ParseError> {
        let params_encoded = self.params.to_string();
        let url_encoded = urlencoding::encode(self.target.as_ref());
        let combined_encoded = format!("{params_encoded}{url_encoded}");
        let signature = self.sign(combined_encoded.as_bytes());

        self.base
            .join(&format!("{signature}/{params_encoded}/{url_encoded}"))
    }
}

/// Builder for [`SignedUrl`].
#[derive(Debug)]
pub struct SignedUrlBuilder<K, B, P, T> {
    key: K,
    base: B,
    params: P,
    target: T,
}

impl SignedUrlBuilder<(), (), (), ()> {
    /// Create a new [`SignedUrlBuilder`].
    pub const fn new() -> Self {
        Self {
            key: (),
            base: (),
            params: (),
            target: (),
        }
    }

    /// Set signing key.
    pub const fn key(self, key: Key) -> SignedUrlBuilder<Key, (), (), ()> {
        let Self {
            base,
            params,
            target,
            ..
        } = self;
        SignedUrlBuilder {
            key,
            base,
            params,
            target,
        }
    }
}

impl SignedUrlBuilder<Key, (), (), ()> {
    /// Set base URL.
    pub const fn base(self, base: Url) -> SignedUrlBuilder<Key, Url, (), ()> {
        let Self {
            key,
            params,
            target,
            ..
        } = self;
        SignedUrlBuilder {
            key,
            base,
            params,
            target,
        }
    }
}

impl SignedUrlBuilder<Key, Url, (), ()> {
    /// Returns a builder on which parameters may be set.
    pub fn params(self) -> SignedUrlBuilder<Key, Url, TransformationParams, ()> {
        let Self {
            key, base, target, ..
        } = self;
        let params = TransformationParams::default();
        SignedUrlBuilder {
            key,
            base,
            target,
            params,
        }
    }
}

impl SignedUrlBuilder<Key, Url, TransformationParams, ()> {
    /// Set resize height.
    pub fn height(self, height: Height) -> Self {
        let Self {
            key,
            base,
            target,
            mut params,
            ..
        } = self;
        params.height = Some(height);
        SignedUrlBuilder {
            key,
            base,
            target,
            params,
        }
    }

    /// Set resize width.
    pub fn width(self, width: Width) -> Self {
        let Self {
            key,
            base,
            target,
            mut params,
            ..
        } = self;
        params.width = Some(width);
        SignedUrlBuilder {
            key,
            base,
            target,
            params,
        }
    }

    /// Set image target URL.
    pub fn target(self, target: Url) -> SignedUrlBuilder<Key, Url, TransformationParams, Url> {
        let Self {
            key, base, params, ..
        } = self;
        SignedUrlBuilder {
            key,
            base,
            target,
            params,
        }
    }
}

impl SignedUrlBuilder<Key, Url, TransformationParams, Url> {
    /// Returns a [`SignedUrl`].
    pub fn build(self) -> SignedUrl {
        SignedUrl::new(self.key, self.base, self.target, self.params)
    }
}

impl Default for SignedUrlBuilder<(), (), (), ()> {
    fn default() -> Self {
        Self::new()
    }
}
