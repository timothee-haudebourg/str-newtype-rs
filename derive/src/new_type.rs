use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

use crate::{
	Error,
	attribute::extract_attributes,
	options::{Derive, ForeignOptions, Options, OwnedTypeOptions},
	utils::SnakeCase,
};

pub fn derive(input: syn::DeriveInput) -> Result<TokenStream, Error> {
	match input.data {
		syn::Data::Struct(s) => match s.fields {
			syn::Fields::Unnamed(unnamed) => {
				let mut fields = unnamed.unnamed.into_iter();

				let field = fields.next().unwrap();
				match &field.ty {
					syn::Type::Path(p) if p.path.is_ident("str") => (),
					_ => return Err(Error::ExpectedStr(field.ty.span())),
				}

				if let Some(u) = fields.next() {
					return Err(Error::UnexpectedField(u.span()));
				}

				let mut options = Options::default();

				extract_attributes(&input.attrs, |attrs| {
					for attr in attrs.0 {
						options.apply(attr)?;
					}

					Ok(())
				})?;

				Ok(derive_with_options(input.ident, &options))
			}
			syn::Fields::Unit => Err(Error::UnexpectedUnitStruct(input.ident.span())),
			syn::Fields::Named(_) => Err(Error::UnexpectedNamedFields(input.ident.span())),
		},
		syn::Data::Enum(_) => Err(Error::UnexpectedEnum(input.ident.span())),
		syn::Data::Union(_) => Err(Error::UnexpectedUnion(input.ident.span())),
	}
}

fn derive_with_options(ident: syn::Ident, options: &Options) -> TokenStream {
	let error = (!options.infallible).then(|| format_ident!("Invalid{ident}"));

	let debug_name = ident.to_string();
	let name = options.name(&ident);

	let new_method_link = format!("[`{ident}::new`]");

	let deref = (!options.no_deref).then(|| {
		quote! {
			impl ::core::ops::Deref for #ident {
				type Target = str;

				fn deref(&self) -> &str {
					&self.0
				}
			}
		}
	});

	let owned_type = options
		.owned
		.as_ref()
		.map(|owned| derive_owned_type(&name, &ident, owned, &options.foreign, error.as_ref()));

	let eq = options
		.foreign
		.eq
		.iter()
		.chain(&options.foreign.ord)
		.map(|ty| partial_eq_impl(&ident, ty, !options.infallible));

	let ord = options
		.foreign
		.ord
		.iter()
		.map(|ty| partial_ord_impl(&ident, ty, !options.infallible));

	let serialize = options.foreign.serde.then(|| {
		quote! {
			impl ::serde::Serialize for #ident {
				fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
				where
					S: ::serde::ser::Serializer
				{
					<str as ::serde::Serialize>::serialize(self.as_str(), serializer)
				}
			}
		}
	});

	let deserialize = options.foreign.serde.then(|| {
		if error.is_some() {
			quote! {
				impl<'a, 'de> ::serde::Deserialize<'de> for &'a #ident where 'de: 'a {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::de::Deserializer<'de>
					{
						#ident::from_str(<&'a str as ::serde::Deserialize<'de>>::deserialize(deserializer)?)
							.map_err(::serde::de::Error::custom)
					}
				}
			}
		} else {
			quote! {
				impl<'a, 'de> ::serde::Deserialize<'de> for &'a #ident where 'de: 'a {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::de::Deserializer<'de>
					{
						<&'a str as ::serde::Deserialize<'de>>::deserialize(deserializer).map(#ident::from_str)?
					}
				}
			}
		}
	});

	let constructor = match error {
		Some(error) => {
			quote! {
				/// Invalid
				#[doc = #name]
				/// error.
				///
				/// This error is raised by the
				#[doc = #new_method_link]
				/// when the input is not a valid
				#[doc = concat!(#name, ".")]
				pub struct #error<T = String>(pub T);

				impl<T: ::core::fmt::Debug> ::core::fmt::Debug for #error<T> {
					fn fmt(&self, f: &mut core::fmt::Formatter) -> ::core::fmt::Result {
						f.write_str(#debug_name)?;
						write!(f, "(")?;
						self.0.fmt(f)?;
						write!(f, ")")
					}
				}

				impl<T: ::core::fmt::Display> ::core::fmt::Display for #error<T> {
					fn fmt(&self, f: &mut core::fmt::Formatter) -> ::core::fmt::Result {
						write!(f, "invalid ")?;
						f.write_str(#name)?;
						write!(f, ": ")?;
						self.0.fmt(f)
					}
				}

				impl<T: ::core::fmt::Debug + ::core::fmt::Display> ::core::error::Error for #error<T> {}

				impl #ident {
					/// Creates a new
					#[doc = #name]
					/// by parsing the input value.
					pub fn new<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<&Self, #error<&T>> {
						let bytes = input.as_ref();
						if Self::validate_bytes(bytes) {
							Ok(unsafe {
								Self::new_unchecked_from_bytes(bytes)
							})
						} else {
							Err(#error(input))
						}
					}

					/// Creates a new
					#[doc = #name]
					/// by parsing the input bytes.
					pub const fn from_bytes(input: &[u8]) -> Result<&Self, #error<&[u8]>> {
						if Self::validate_bytes(input) {
							Ok(unsafe {
								Self::new_unchecked_from_bytes(input)
							})
						} else {
							Err(#error(input))
						}
					}

					/// Creates a new
					#[doc = #name]
					/// by parsing the input string.
					pub const fn from_str(input: &str) -> Result<&Self, #error<&str>> {
						if Self::validate_str(input) {
							Ok(unsafe {
								Self::new_unchecked(input)
							})
						} else {
							Err(#error(input))
						}
					}

					/// Creates a new
					#[doc = #name]
					/// from the input bytes without validation.
					///
					/// # Safety
					/// The input bytes must be a valid
					#[doc = concat!(#name, ".")]
					pub const unsafe fn new_unchecked_from_bytes(input: &[u8]) -> &Self {
						unsafe { std::mem::transmute::<&[u8], &Self>(input) }
					}

					/// Creates a new
					#[doc = #name]
					/// from the input string without validation.
					///
					/// # Safety
					/// The input string must be a valid
					#[doc = concat!(#name, ".")]
					pub const unsafe fn new_unchecked(input: &str) -> &Self {
						unsafe { Self::new_unchecked_from_bytes(input.as_bytes()) }
					}
				}

				impl<'a> TryFrom<&'a [u8]> for &'a #ident {
					type Error = #error<&'a [u8]>;

					fn try_from(value: &'a[u8]) -> Result<&'a #ident, #error<&'a [u8]>> {
						#ident::new(value)
					}
				}

				impl<'a> TryFrom<&'a str> for &'a #ident {
					type Error = #error<&'a str>;

					fn try_from(value: &'a str) -> Result<&'a #ident, #error<&'a str>> {
						#ident::new(value)
					}
				}
			}
		}
		None => {
			quote! {
				impl #ident {
					/// Creates a new
					#[doc = #name]
					/// by parsing the input value.
					pub fn new<T: ?Sized + AsRef<str>>(input: &T) -> &Self {
						Self::from_str(input.as_ref())
					}

					/// Creates a new
					#[doc = #name]
					/// by parsing the input bytes.
					pub const fn from_bytes(input: &[u8]) -> Result<&Self, ::std::str::Utf8Error> {
						match ::std::str::from_utf8(input) {
							Ok(s) => Ok(unsafe { Self::from_str(s) }),
							Err(e) => Err(e)
						}
					}

					/// Creates a new
					#[doc = #name]
					/// by parsing the input string.
					pub const fn from_str(input: &str) -> &Self {
						unsafe { std::mem::transmute::<&str, &Self>(input) }
					}
				}

				impl<'a> TryFrom<&'a [u8]> for &'a #ident {
					type Error = ::std::str::Utf8Error;

					fn try_from(value: &'a[u8]) -> Result<&'a #ident, ::std::str::Utf8Error> {
						#ident::from_bytes(value)
					}
				}

				impl<'a> From<&'a str> for &'a #ident {
					fn from(value: &'a str) -> &'a #ident {
						#ident::new(value)
					}
				}
			}
		}
	};

	quote! {
		#constructor

		impl #ident {
			/// Returns the
			#[doc = #name]
			/// as a string.
			pub const fn as_str(&self) -> &str {
				&self.0
			}

			/// Returns the
			#[doc = #name]
			/// as a byte string.
			pub const fn as_bytes(&self) -> &[u8] {
				self.0.as_bytes()
			}
		}

		impl AsRef<#ident> for #ident {
			fn as_ref(&self) -> &#ident {
				self
			}
		}

		impl AsRef<str> for #ident {
			fn as_ref(&self) -> &str {
				self.as_str()
			}
		}

		impl AsRef<[u8]> for #ident {
			fn as_ref(&self) -> &[u8] {
				self.as_str().as_bytes()
			}
		}

		impl ::core::fmt::Display for #ident {
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				f.write_str(self.as_str())
			}
		}

		impl ::core::fmt::Debug for #ident {
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				f.write_str(self.as_str())
			}
		}

		impl ::core::borrow::Borrow<str> for #ident {
			fn borrow(&self) -> &str {
				self.as_str()
			}
		}

		impl<'a> From<&'a #ident> for &'a str {
			fn from(value: &'a #ident) -> Self {
				value.as_str()
			}
		}

		impl<'a> From<&'a #ident> for &'a [u8] {
			fn from(value: &'a #ident) -> Self {
				value.as_bytes()
			}
		}

		#deref

		#(#eq)*

		#(#ord)*

		#serialize

		#deserialize

		#owned_type
	}
}

fn partial_eq_impl(ident: &syn::Ident, ty: &syn::Type, fallible: bool) -> TokenStream {
	if fallible {
		quote! {
			impl PartialEq<#ty> for #ident {
				fn eq(&self, other: &#ty) -> bool {
					match Self::new(other) {
						Ok(other) => self == other,
						Err(_) => false
					}
				}
			}

			impl PartialEq<#ident> for #ty {
				fn eq(&self, other: &#ident) -> bool {
					match #ident::new(self) {
						Ok(this) => this == other,
						Err(_) => false
					}
				}
			}
		}
	} else {
		quote! {
			impl PartialEq<#ty> for #ident {
				fn eq(&self, other: &#ty) -> bool {
					self == Self::new(other)
				}
			}

			impl PartialEq<#ident> for #ty {
				fn eq(&self, other: &#ident) -> bool {
					#ident::new(self) == other
				}
			}
		}
	}
}

fn partial_ord_impl(ident: &syn::Ident, ty: &syn::Type, fallible: bool) -> TokenStream {
	if fallible {
		quote! {
			impl PartialOrd<#ty> for #ident {
				fn partial_cmp(&self, other: &#ty) -> Option<::core::cmp::Ordering> {
					match Self::new(other) {
						Ok(other) => self.partial_cmp(other),
						Err(_) => None
					}
				}
			}

			impl PartialOrd<#ident> for #ty {
				fn partial_cmp(&self, other: &#ident) -> Option<::core::cmp::Ordering> {
					match #ident::new(self) {
						Ok(this) => this.partial_cmp(other),
						Err(_) => None
					}
				}
			}
		}
	} else {
		quote! {
			impl PartialOrd<#ty> for #ident {
				fn partial_cmp(&self, other: &#ty) -> Option<::core::cmp::Ordering> {
					self.partial_cmp(Self::new(other))
				}
			}

			impl PartialOrd<#ident> for #ty {
				fn partial_cmp(&self, other: &#ident) -> Option<::core::cmp::Ordering> {
					#ident::new(self).partial_cmp(other)
				}
			}
		}
	}
}

fn derive_owned_type(
	name: &str,
	ident: &syn::Ident,
	options: &OwnedTypeOptions,
	foreign: &ForeignOptions,
	error: Option<&syn::Ident>,
) -> TokenStream {
	let as_ref = format_ident!("as_{}", SnakeCase(&ident.to_string()));
	let owned_ident = &options.ident;

	let derives = options
		.derives
		.iter()
		.map(|d| d.generate(ident, owned_ident, &as_ref, foreign));

	let constructor = match error {
		Some(error) => quote! {
			impl #owned_ident {
				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input value.
				pub fn new<T: str_newtype::Buffer>(input: T) -> Result<Self, #error<T>> {
					if #ident::validate_bytes(input.as_bytes()) {
						Ok(unsafe {
							Self::new_unchecked(input.into_bytes())
						})
					} else {
						Err(#error(input))
					}
				}

				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input bytes.
				pub fn from_bytes(input: Vec<u8>) -> Result<Self, #error<Vec<u8>>> {
					Self::new(input)
				}

				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input string.
				pub fn from_string(input: String) -> Result<Self, #error> {
					Self::new(input)
				}

				/// Creates a new owned
				#[doc = #name]
				/// from the input value without validation.
				///
				/// # Safety
				/// The input value must be a valid
				#[doc = concat!(#name, ".")]
				pub unsafe fn new_unchecked(input: impl Into<Vec<u8>>) -> Self {
					Self(unsafe {
						String::from_utf8_unchecked(input.into())
					})
				}

				pub const fn #as_ref(&self) -> &#ident {
					unsafe {
						#ident::new_unchecked(self.0.as_str())
					}
				}
			}

			impl TryFrom<Vec<u8>> for #owned_ident {
				type Error = #error<Vec<u8>>;

				fn try_from(value: Vec<u8>) -> Result<Self, #error<Vec<u8>>> {
					Self::new(value)
				}
			}

			impl TryFrom<String> for #owned_ident {
				type Error = #error;

				fn try_from(value: String) -> Result<Self, #error> {
					Self::new(value)
				}
			}

			impl ::std::str::FromStr for #owned_ident {
				type Err = #error;

				fn from_str(value: &str) -> Result<Self, #error> {
					Self::new(value.to_owned())
				}
			}
		},
		None => quote! {
			impl #owned_ident {
				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input value.
				pub fn new(input: impl Into<String>) -> Self {
					Self(input.into())
				}

				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input string.
				pub fn from_string(input: String) -> Self {
					Self(input)
				}

				/// Creates a new owned
				#[doc = #name]
				/// by parsing the input bytes.
				pub fn from_bytes(input: Vec<u8>) -> Result<Self, ::std::string::FromUtf8Error> {
					Ok(Self::new(String::from_utf8(input)?))
				}

				pub fn #as_ref(&self) -> &#ident {
					#ident::new(self.0.as_str())
				}
			}

			impl TryFrom<Vec<u8>> for #owned_ident {
				type Error = ::std::string::FromUtf8Error;

				fn try_from(value: Vec<u8>) -> Result<Self, ::std::string::FromUtf8Error> {
					Self::from_bytes(value)
				}
			}

			impl From<String> for #owned_ident {
				fn from(value: String) -> Self {
					Self(value)
				}
			}

			impl ::std::str::FromStr for #owned_ident {
				type Err = ::std::convert::Infallible;

				fn from_str(value: &str) -> Result<Self, ::std::convert::Infallible> {
					Ok(Self(value.to_owned()))
				}
			}
		},
	};

	let serialize = foreign.serde.then(|| {
		quote! {
			impl ::serde::Serialize for #owned_ident {
				fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
				where
					S: ::serde::ser::Serializer
				{
					<str as ::serde::Serialize>::serialize(self.as_str(), serializer)
				}
			}
		}
	});

	let deserialize = foreign.serde.then(|| {
		if error.is_some() {
			quote! {
				impl<'de> ::serde::Deserialize<'de> for #owned_ident {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::de::Deserializer<'de>
					{
						#owned_ident::new(<String as ::serde::Deserialize<'de>>::deserialize(deserializer)?)
							.map_err(::serde::de::Error::custom)
					}
				}
			}
		} else {
			quote! {
				impl<'de> ::serde::Deserialize<'de> for #owned_ident {
					fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
					where
						D: ::serde::de::Deserializer<'de>
					{
						<String as ::serde::Deserialize<'de>>::deserialize(deserializer).map(Self)?
					}
				}
			}
		}
	});

	let vis = error.is_none().then(|| quote! { pub });

	quote! {
		/// Owned
		#[doc = concat!(#name, ".")]
		#[derive(Clone)]
		pub struct #owned_ident(#vis String);

		#constructor

		impl #owned_ident {
			/// Returns the
			#[doc = #name]
			/// as a string.
			pub fn as_str(&self) -> &str {
				self.0.as_str()
			}

			/// Returns the
			#[doc = #name]
			/// as a byte string.
			pub fn as_bytes(&self) -> &[u8] {
				self.0.as_bytes()
			}

			pub fn into_string(self) -> String {
				self.0
			}

			pub fn into_bytes(self) -> Vec<u8> {
				self.0.into_bytes()
			}
		}

		impl ::std::borrow::Borrow<#ident> for #owned_ident {
			fn borrow(&self) -> &#ident {
				self.#as_ref()
			}
		}

		impl ::std::borrow::ToOwned for #ident {
			type Owned = #owned_ident;

			fn to_owned(&self) -> Self::Owned {
				#owned_ident(self.as_str().to_owned())
			}
		}

		impl ::core::ops::Deref for #owned_ident {
			type Target = #ident;

			fn deref(&self) -> &Self::Target {
				self.#as_ref()
			}
		}

		impl AsRef<#ident> for #owned_ident {
			fn as_ref(&self) -> &#ident {
				self.#as_ref()
			}
		}

		impl AsRef<str> for #owned_ident {
			fn as_ref(&self) -> &str {
				self.as_str()
			}
		}

		impl AsRef<[u8]> for #owned_ident {
			fn as_ref(&self) -> &[u8] {
				self.as_str().as_bytes()
			}
		}

		impl ::core::fmt::Debug for #owned_ident {
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				<#ident as ::core::fmt::Debug>::fmt(
					self.#as_ref(),
					f
				)
			}
		}

		impl ::core::fmt::Display for #owned_ident {
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				<#ident as ::core::fmt::Display>::fmt(
					self.#as_ref(),
					f
				)
			}
		}

		impl From<#owned_ident> for String {
			fn from(value: #owned_ident) -> Self {
				value.into_string()
			}
		}

		impl From<#owned_ident> for Vec<u8> {
			fn from(value: #owned_ident) -> Self {
				value.into_bytes()
			}
		}

		#serialize

		#deserialize

		#(#derives)*
	}
}

fn owned_partial_eq_impl(
	owned_ident: &syn::Ident,
	as_ref: &syn::Ident,
	ty: &syn::Type,
) -> TokenStream {
	quote! {
		impl PartialEq<#ty> for #owned_ident {
			fn eq(&self, other: &#ty) -> bool {
				self.#as_ref() == other
			}
		}

		impl PartialEq<#owned_ident> for #ty {
			fn eq(&self, other: &#owned_ident) -> bool {
				self == other.#as_ref()
			}
		}
	}
}

fn owned_partial_ord_impl(
	owned_ident: &syn::Ident,
	as_ref: &syn::Ident,
	ty: &syn::Type,
) -> TokenStream {
	quote! {
		impl PartialOrd<#ty> for #owned_ident {
			fn partial_cmp(&self, other: &#ty) -> Option<::core::cmp::Ordering> {
				self.#as_ref().partial_cmp(other)
			}
		}

		impl PartialOrd<#owned_ident> for #ty {
			fn partial_cmp(&self, other: &#owned_ident) -> Option<::core::cmp::Ordering> {
				self.partial_cmp(other.#as_ref())
			}
		}
	}
}

impl Derive {
	fn generate(
		&self,
		ident: &syn::Ident,
		owned_ident: &syn::Ident,
		as_ref: &syn::Ident,
		foreign: &ForeignOptions,
	) -> TokenStream {
		match self {
			Self::Default => {
				quote! {
					impl ::core::default::Default for #owned_ident {
						fn default() -> Self {
							<&'static #ident as ::core::default::Default>::default().to_owned()
						}
					}
				}
			}
			Self::PartialEq => {
				let foreign = foreign
					.eq
					.iter()
					.chain(&foreign.ord)
					.map(|ty| owned_partial_eq_impl(owned_ident, as_ref, ty));

				quote! {
					impl PartialEq for #owned_ident {
						fn eq(&self, other: &Self) -> bool {
							<#ident as PartialEq>::eq(
								self.#as_ref(),
								other.#as_ref()
							)
						}
					}

					impl PartialEq<#ident> for #owned_ident {
						fn eq(&self, other: &#ident) -> bool {
							<#ident as PartialEq>::eq(
								self.#as_ref(),
								other
							)
						}
					}

					impl PartialEq<&#ident> for #owned_ident {
						fn eq(&self, other: &&#ident) -> bool {
							<#ident as PartialEq>::eq(
								self.#as_ref(),
								*other
							)
						}
					}

					impl PartialEq<#owned_ident> for #ident {
						fn eq(&self, other: &#owned_ident) -> bool {
							<#ident as PartialEq>::eq(
								self,
								other.#as_ref()
							)
						}
					}

					impl PartialEq<#owned_ident> for &#ident {
						fn eq(&self, other: &#owned_ident) -> bool {
							<#ident as PartialEq>::eq(
								*self,
								other.#as_ref()
							)
						}
					}

					#(#foreign)*
				}
			}
			Self::Eq => {
				quote! {
					impl Eq for #owned_ident {}
				}
			}
			Self::PartialOrd => {
				let foreign = foreign
					.ord
					.iter()
					.map(|ty| owned_partial_ord_impl(owned_ident, as_ref, ty));

				quote! {
					impl PartialOrd for #owned_ident {
						fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
							<#ident as PartialOrd>::partial_cmp(
								self.#as_ref(),
								other.#as_ref()
							)
						}
					}

					impl PartialOrd<#ident> for #owned_ident {
						fn partial_cmp(&self, other: &#ident) -> Option<::core::cmp::Ordering> {
							<#ident as PartialOrd>::partial_cmp(
								self.#as_ref(),
								other
							)
						}
					}

					impl PartialOrd<&#ident> for #owned_ident {
						fn partial_cmp(&self, other: &&#ident) -> Option<::core::cmp::Ordering> {
							<#ident as PartialOrd>::partial_cmp(
								self.#as_ref(),
								*other
							)
						}
					}

					impl PartialOrd<#owned_ident> for #ident {
						fn partial_cmp(&self, other: &#owned_ident) -> Option<::core::cmp::Ordering> {
							<#ident as PartialOrd>::partial_cmp(
								self,
								other.#as_ref()
							)
						}
					}

					impl PartialOrd<#owned_ident> for &#ident {
						fn partial_cmp(&self, other: &#owned_ident) -> Option<::core::cmp::Ordering> {
							<#ident as PartialOrd>::partial_cmp(
								*self,
								other.#as_ref()
							)
						}
					}

					#(#foreign)*
				}
			}
			Self::Ord => {
				quote! {
					impl Ord for #owned_ident {
						fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
							<#ident as Ord>::cmp(
								self.#as_ref(),
								other.#as_ref()
							)
						}
					}
				}
			}
			Self::Hash => {
				quote! {
					impl ::core::hash::Hash for #owned_ident {
						fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
							<#ident as ::core::hash::Hash>::hash(
								self.#as_ref(),
								state
							)
						}
					}
				}
			}
		}
	}
}
