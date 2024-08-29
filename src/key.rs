// Copied and adapted from: https://github.com/rwf2/cookie-rs/blob/ba46fc5e97a1271435f38509d109e125a473bc82/src/secure/key.rs
use std::convert::TryFrom;

const KEY_LENGTH: usize = 64;

/// Cryptographically-secure key used for signing URLs.
#[derive(Clone)]
pub struct Key([u8; KEY_LENGTH]);

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;

        self.0.ct_eq(&other.0).into()
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Key").finish()
    }
}

impl Key {
    // An empty key structure, to be filled.
    const fn zero() -> Self {
        Key([0; KEY_LENGTH])
    }

    /// Creates a new `Key` from a 512-bit cryptographically random string.
    ///
    /// The supplied key must be at least 512-bits (64 bytes). For security, the
    /// master key _must_ be cryptographically random.
    ///
    /// # Panics
    ///
    /// Panics if `key` is less than 64 bytes in length.
    ///
    /// For a non-panicking version, use [`Key::try_from()`] or generate a key
    /// with [`Key::generate()`] or [`Key::try_generate()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use tower_image_xform::Key;
    ///
    /// # /*
    /// let key = { /* a cryptographically random key >= 64 bytes */ };
    /// # */
    /// # let key: &Vec<u8> = &(0..64).collect();
    ///
    /// let key = Key::from(key);
    /// ```
    #[inline]
    pub fn from(key: &[u8]) -> Key {
        Key::try_from(key).unwrap()
    }

    /// Generates signing/encryption keys from a secure, random source. Keys are
    /// generated nondeterministically.
    ///
    /// # Panics
    ///
    /// Panics if randomness cannot be retrieved from the operating system. See
    /// [`Key::try_generate()`] for a non-panicking version.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tower_image_xform::Key;
    ///
    /// let key = Key::generate();
    /// ```
    pub fn generate() -> Key {
        Self::try_generate().expect("failed to generate `Key` from randomness")
    }

    /// Attempts to generate signing/encryption keys from a secure, random
    /// source. Keys are generated nondeterministically. If randomness cannot be
    /// retrieved from the underlying operating system, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tower_image_xform::Key;
    ///
    /// let key = Key::try_generate();
    /// ```
    pub fn try_generate() -> Option<Key> {
        use rand::RngCore;

        let mut rng = rand::thread_rng();
        let mut key = Key::zero();
        rng.try_fill_bytes(&mut key.0).ok()?;
        Some(key)
    }

    /// Returns the key as a slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum KeyError {
    /// Too few bytes (`.0`) were provided to generate a key.
    ///
    /// See [`Key::from()`] for minimum requirements.
    TooShort(usize),
}

impl std::error::Error for KeyError {}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::TooShort(n) => {
                write!(
                    f,
                    "key material is too short: expected >= {} bytes, got {} bytes",
                    KEY_LENGTH, n
                )
            }
        }
    }
}

impl TryFrom<&[u8]> for Key {
    type Error = KeyError;

    /// A fallible version of [`Key::from()`].
    ///
    /// Succeeds when [`Key::from()`] succeds and returns an error where
    /// [`Key::from()`] panics, namely, if `key` is too short.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::convert::TryFrom;
    /// use tower_image_xform::Key;
    ///
    /// # /*
    /// let key = { /* a cryptographically random key >= 64 bytes */ };
    /// # */
    /// # let key: &Vec<u8> = &(0..64).collect();
    /// # let key: &[u8] = &key[..];
    /// assert!(Key::try_from(key).is_ok());
    ///
    /// // A key that's far too short to use.
    /// let key = &[1, 2, 3, 4][..];
    /// assert!(Key::try_from(key).is_err());
    /// ```
    fn try_from(key: &[u8]) -> Result<Self, Self::Error> {
        if key.len() < KEY_LENGTH {
            Err(KeyError::TooShort(key.len()))
        } else {
            let mut output = Key::zero();
            output.0.copy_from_slice(&key[..KEY_LENGTH]);
            Ok(output)
        }
    }
}
