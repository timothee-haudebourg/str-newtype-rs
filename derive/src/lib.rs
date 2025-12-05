//! Derive macro for the [`str-newtype`] library. This crate is not supposed
//! to be used directly.
//!
//! [`str-newtype`]: https://crates.io/crates/str-newtype
use proc_macro_error::{abort, proc_macro_error};
use proc_macro2::Span;
use syn::parse_macro_input;

mod attribute;
mod new_type;
mod options;
mod utils;

/// Derives methods and traits for an `str` new-type, along with an owned
/// companion type.
///
/// This derive macro must be used on a type of the form:
/// ```ignore
/// #[derive(StrNewType)]
/// #[newtype(...)]
/// pub struct Type(str);
/// ```
///
/// The target type must implement two `const` compatible validation methods:
/// - `validate_bytes(&[u8]) -> bool`; and
/// - `validate_str(&str) -> bool`.
///
/// The macro will then derive various methods, types and trait implementations
/// depending on the sub-attributes specified in the `newtype` attribute.
///
/// # Derived items
///
/// Here is the full list of the derived items, depending on the sub-attributes
/// passed to the `newtype` attribute.
///
/// - If the `infallible` sub-attribute is *not* set:
///   - Error type `Invalid{Type}<T = String>(pub T);` with
///     - `Debug` implementation
///     - `Display` implementation
///     - `Error` implementation
///   - `Type::new<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<&Self, Invalid{Type}<&T>>` constructor;
///   - `const Type::from_bytes(input: &[u8]) -> Result<&Self, Invalid{Type}<&[u8]>>` constructor;
///   - `const Type::from_str(input: &str) -> Result<&Str, Invalid{Type}<&str>>` constructor;
///   - `&Type: TryFrom<&str>`
/// - If the `infallible` sub-attribute is set:
///   - `Type::new<T: ?Sized + AsRef<[u8]>>(input: &T) -> &Self` constructor;
///   - `const Type::from_bytes(input: &[u8]) -> &Self` constructor;
///   - `const Type::from_str(input: &str) -> &Self` constructor;
///   - `&Type: From<&str>`
/// - `&Type: TryFrom<&[u8]>`
/// - `Type: AsRef<Self>`
/// - `Type: AsRef<str>`
/// - `Type: AsRef<[u8]>`
/// - `Type: Display`
/// - `Type: Debug`
/// - `Type: Borrow<str>`
/// - `&str: From<&Type>`
/// - `&[u8]: From<&Type>`
/// - If the `noderef` sub-attribute is *not* set:
///   - `Type: Deref<str>`
/// - If the `eq(Other)` attribute is set:
///   - `Type: PartialEq<Other>`
///   - `Other: PartialEq<Type>`
/// - If the `ord(Other)` attribute is set:
///   - `Type: PartialOrd<Other>`
///   - `Other: PartialOrd<Type>`
/// - If the `serde` attribute is set:
///   - `Type: ::serde::Serialize`
///   - `&Type: ::serde::Deserialize<'_>`
/// - If the `owned(OwnedType, ...)` sub-attribute is set (where `...` denotes
///   the owned-type sub-attributes):
///   - If the `infallible` sub-attribute is *not* set:
///     - `struct OwnedType(String)`
///     - `OwnedType::new<T: str_newtype::Buffer>(input: T) -> Result<Self, Invalid{Type}<T>>`
///     - `OwnedType::from_bytes(input: Vec<u8>) -> Result<Self, Invalid{Type}<Vec<u8>>>`
///     - `OwnedType::from_string(input: String) -> Result<Self, Invalid{Type}>`
///     - `unsafe OwnedType::new_unchecked(input: impl Into<Vec<u8>>) -> Self`
///     - `OwnedType: TryFrom<String>`
///   - If the `infallible` sub-attribute is set:
///     - `struct OwnedType(pub String)`
///     - `OwnedType::new(input: impl Into<String>) -> Self`
///     - `OwnedType::from_string(input: String) -> Self`
///     - `OwnedType::from_bytes(input: Vec<u8>) -> Result<Self, ::std::string::FromUtf8Error>`
///     - `OwnedType: From<String>`
///   - `OwnedType: Display`
///   - `OwnedType: Debug`
///   - `OwnedType: Clone`
///   - `OwnedType: FromStr`
///   - `Type: ToOwned<Owned => OwnedType>`
///   - `OwnedType: Deref<Target = Type>`
///   - `OwnedType: TryFrom<Vec<u8>>`
///   - `OwnedType::as_{type}(&self) -> &Type` where `{type}` is the camel case version of `Type`.
///   - `OwnedType::as_str(&self) -> &str`
///   - `OwnedType::as_bytes(&self) -> &[u8]`
///   - `OwnedType::into_string(self) -> String`
///   - `OwnedType::into_bytes(self) -> Vec<u8>`
///   - `OwnedType: Borrow<Type>`
///   - `OwnedType: AsRef<Type>`
///   - `OwnedType: AsRef<str>`
///   - `OwnedType: AsRef<[u8]>`
///   - `String: From<OwnedType>`
///   - `Vec<u8>: From<OwnedType>`
///   - If the `eq(Other)` attribute is set:
///     - `OwnedType: PartialEq<Other>`
///     - `Other: PartialEq<OwnedType>`
///   - If the `ord(Other)` attribute is set:
///     - `OwnedType: PartialOrd<Other>`
///     - `Other: PartialOrd<OwnedType>`
///   - If the `serde` attribute is set:
///     - `OwnedType: ::serde::Serialize`
///     - `OwnedType: ::serde::Deserialize<'_>`
///   - If the `derive(Default)` owned-type sub-attribute is set:
///     - `OwnedType: Default` (requires `Type: Default`)
///   - If the `derive(PartialEq)` owned-type sub-attribute is set:
///     - `OwnedType: PartialEq` (requires `Type: PartialEq`)
///   - If the `derive(Eq)` owned-type sub-attribute is set:
///     - `OwnedType: Eq` (requires `Type: Eq`)
///   - If the `derive(PartialOrd)` owned-type sub-attribute is set:
///     - `OwnedType: PartialOrd` (requires `Type: PartialOrd`)
///   - If the `derive(Ord)` owned-type sub-attribute is set:
///     - `OwnedType: Ord` (requires `Type: Ord`)
///   - If the `derive(Hash)` owned-type sub-attribute is set:
///     - `OwnedType: Hash` (requires `Type: Hash`)
///
/// # The `newtype` attribute
///
/// Generated items can be configured using the `newtype` attribute.
/// This attribute takes sub-attribute between parenthesis. For example:
///
/// ```ignore
/// #[derive(StrNewType)]
/// #[newtype(eq(str, [u8]), ord(str), noderef, owned(Foo, derive(Default, Hash)))]
/// pub struct Type(str);
/// ```
///
/// Here is the list of sub-attributes:
/// - `noderef`: Prevent the `Type: Deref<Target = str>` implementation.
/// - `eq`: Implement `Type: PartialEq<Other>` (and
///   `OwnedType: PartialEq<Other>` if applicable) where `Other` must appear in
///   a parenthesized comma-separated list after the sub-attribute
///   (`eq(A, B, C)`).
/// - `ord`: Implement `Type: PartialOrd<Other>` (and
///   `OwnedType: PartialOrd<Other>` if applicable) where `Other` must appear in
///   a parenthesized comma-separated list after the sub-attribute
///   (`ord(A, B, C)`).
/// - `serde`: Implement `Type: Serialize + Deserialize` (and
///   `OwnedType: Serialize + Deserialize` if applicable)
/// - `owned(OwnedType)`: Derive an owned variant of `Type` called `OwnedType`.
///   This sub-attribute can take additional owned-type sub-attributes after the
///   identifier:
///   - `derive`: Specifies the list of trait to derive on `OwnedType`. Must be
///   given as a parenthesized comma-separated list (e.g.
///   `derive(Default, Hash)`). Possible traits are:
///     - `Default`
///     - `PartialEq`
///     - `Eq`
///     - `PartialOrd`
///     - `Ord`
///     - `Hash`
#[proc_macro_derive(StrNewType, attributes(newtype))]
#[proc_macro_error]
pub fn derive_regular_grammar(input_tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input_tokens as syn::DeriveInput);
	match new_type::derive(input) {
		Ok(tokens) => tokens.into(),
		Err(e) => {
			let span = e.span();
			abort!(span, e)
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("unexpected enum type")]
	UnexpectedEnum(Span),

	#[error("unexpected union type")]
	UnexpectedUnion(Span),

	#[error("unexpected unit struct")]
	UnexpectedUnitStruct(Span),

	#[error("unexpected named fields")]
	UnexpectedNamedFields(Span),

	#[error("unexpected field")]
	UnexpectedField(Span),

	#[error("expected `str` type")]
	ExpectedStr(Span),

	#[error("invalid attribute")]
	InvalidAttribute(Span),

	#[error(transparent)]
	Syn(#[from] syn::Error),
}

impl Error {
	pub fn span(&self) -> Span {
		match self {
			Self::UnexpectedEnum(s) => *s,
			Self::UnexpectedUnion(s) => *s,
			Self::UnexpectedUnitStruct(s) => *s,
			Self::UnexpectedNamedFields(s) => *s,
			Self::UnexpectedField(s) => *s,
			Self::ExpectedStr(s) => *s,
			Self::InvalidAttribute(s) => *s,
			Self::Syn(e) => e.span(),
		}
	}
}
