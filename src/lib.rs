//! This library provides a handy derive macro to create type-safe wrappers
//! around borrowed and owned string types (`str` and `String`), guaranteeing
//! through the type system that those string have been validated.
//!
//! With this derive macro, all you need to do is define the base borrowed type,
//! specify the name of the associated owned type and what trait you want them
//! to implement.
//!
//! # Example
//!
//! In this example, we wish to define the types `FooStr` and `FooString`,
//! similar to `str` (borrowed) and `String` (owned), with the extra guarantee
//! that the string is always equal to `"foo"`.
//!
//! ```
//! use str_newtype::StrNewType;
//!
//! /// An `str` that is equal to `"foo"`.
//! #[derive(StrNewType)]
//! #[newtype(owned(FooString))]
//! pub struct FooStr(str);
//!
//! impl FooStr {
//!   pub const fn validate_bytes(s: &[u8]) -> bool {
//!     s.len() == 3 && s[0] == b'f' && s[1] == b'f' && s[2] == b'f'
//!   }
//!
//!   pub const fn validate_str(s: &str) -> bool {
//!     Self::validate_bytes(s.as_bytes())
//!   }
//! }
//! ```
//!
//! The validation methods (`validate_*`) are provided by us, but `StrNewType`
//! will use them to derive the following:
//! - An error type `InvalidFooStr<T = String>(pub T)`;
//! - A constructor `FooStr::new<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<&Self, InvalidFooStr<&T>>`
//! - `FooStr: Deref<Target = str>` implementation
//! - `FooStr: AsRef<str>` implementation
//! - `FooStr: Borrow<str>` implementation
//!
//! Since we added the `owned(FooString)` attribute, it will also generate:
//! - A sized/owned version of `FooStr`: `struct FooString(String);`
//! - A constructor `FooString::new<T: Buffer>(s: T) -> Result<Self, InvalidFooStr<T>>`
//! - `FooString: Deref<Target = FooStr>`
//! - `FooString: AsRef<FooStr>` implementation
//! - `FooString: Borrow<FooStr>` implementation
//!
//! And much more. See the the [`StrNewType`] documentation for a full
//! specification of what items are derived and how it can be controlled with
//! the `newtype` attribute.
pub use str_newtype_derive::StrNewType;

/// Trusted byte buffer type.
///
/// # Safety
///
/// Any interior mutability in the buffer type must not affect the `as_bytes`
/// and `into_bytes` methods. In other words, as long as `self` is borrowed
/// immutably those functions must always return the same result.
pub unsafe trait Buffer: Sized {
	/// Borrows the buffer bytes.
	fn as_bytes(&self) -> &[u8];

	/// Turns this buffer into a byte array.
	fn into_bytes(self) -> Vec<u8>;
}

unsafe impl Buffer for Vec<u8> {
	fn as_bytes(&self) -> &[u8] {
		self
	}

	fn into_bytes(self) -> Vec<u8> {
		self
	}
}

unsafe impl Buffer for String {
	fn as_bytes(&self) -> &[u8] {
		self.as_bytes()
	}

	fn into_bytes(self) -> Vec<u8> {
		self.into_bytes()
	}
}
