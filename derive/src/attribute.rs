use crate::{options::Derive, Error};
use syn::{
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
	spanned::Spanned,
	Token,
};

pub fn extract_attributes(
	attrs: &[syn::Attribute],
	mut f: impl FnMut(Attributes) -> Result<(), Error>,
) -> Result<(), Error> {
	for attr in attrs {
		if attr.meta.path().is_ident("newtype") {
			match &attr.meta {
				syn::Meta::List(m) => {
					let newtype_attrs: Attributes = syn::parse2(m.tokens.clone())?;
					f(newtype_attrs)?
				}
				_ => return Err(Error::InvalidAttribute(attr.span())),
			}
		}
	}

	Ok(())
}

pub struct Attributes(pub Punctuated<Attribute, Token![,]>);

impl Parse for Attributes {
	fn parse(input: ParseStream) -> syn::parse::Result<Self> {
		Punctuated::parse_terminated(input).map(Self)
	}
}

pub enum Attribute {
	Name(syn::LitStr),
	Owned(Punctuated<OwnedTypeAttribute, Token![,]>),
	Eq(Punctuated<syn::Type, Token![,]>),
	Ord(Punctuated<syn::Type, Token![,]>),
	Serde,
	NoDeref,
	Infallible,
}

impl Parse for Attribute {
	fn parse(input: ParseStream) -> syn::parse::Result<Self> {
		let ident: syn::Ident = input.parse()?;

		if ident == "no_deref" {
			return Ok(Self::NoDeref);
		}

		if ident == "infallible" {
			return Ok(Self::Infallible);
		}

		if ident == "name" {
			let _: Token![=] = input.parse()?;
			return input.parse().map(Self::Name);
		}

		if ident == "owned" {
			let content;
			syn::parenthesized!(content in input);
			return Punctuated::parse_terminated(&content).map(Self::Owned);
		}

		if ident == "eq" {
			let content;
			syn::parenthesized!(content in input);
			return Punctuated::parse_terminated(&content).map(Self::Eq);
		}

		if ident == "ord" {
			let content;
			syn::parenthesized!(content in input);
			return Punctuated::parse_terminated(&content).map(Self::Ord);
		}

		if ident == "serde" {
			return Ok(Self::Serde);
		}

		Err(syn::parse::Error::new(ident.span(), "unknown attribute"))
	}
}

pub enum OwnedTypeAttribute {
	Ident(syn::Ident),
	Derive(Punctuated<Derive, Token![,]>),
}

impl Parse for OwnedTypeAttribute {
	fn parse(input: ParseStream) -> syn::parse::Result<Self> {
		let ident: syn::Ident = input.parse()?;

		if ident == "derive" {
			let content;
			syn::parenthesized!(content in input);
			return Punctuated::parse_terminated(&content).map(Self::Derive);
		}

		Ok(Self::Ident(ident))
	}
}

impl Parse for Derive {
	fn parse(input: ParseStream) -> syn::parse::Result<Self> {
		let ident: syn::Ident = input.parse()?;

		if ident == "Default" {
			return Ok(Self::Default);
		}

		if ident == "PartialEq" {
			return Ok(Self::PartialEq);
		}

		if ident == "Eq" {
			return Ok(Self::Eq);
		}

		if ident == "PartialOrd" {
			return Ok(Self::PartialOrd);
		}

		if ident == "Ord" {
			return Ok(Self::Ord);
		}

		if ident == "Hash" {
			return Ok(Self::Hash);
		}

		Err(syn::parse::Error::new(ident.span(), "unsupported trait"))
	}
}
