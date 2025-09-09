use crate::{
	attribute::{Attribute, OwnedTypeAttribute},
	Error,
};

#[derive(Default)]
pub struct Options {
	pub name: Option<String>,
	pub owned: Option<OwnedTypeOptions>,
	pub foreign: ForeignOptions,
	pub no_deref: bool,
	pub infallible: bool,
}

impl Options {
	pub fn name(&self, ident: &syn::Ident) -> String {
		self.name
			.clone()
			.unwrap_or_else(|| ident.to_string().to_lowercase())
	}

	pub fn apply(&mut self, attr: Attribute) -> Result<(), Error> {
		match attr {
			Attribute::Name(name) => match &mut self.name {
				Some(n) => n.push_str(&name.value()),
				None => self.name = Some(name.value()),
			},
			Attribute::Owned(attrs) => {
				let mut ident = None;
				let mut derives = Derives::default();

				for attr in attrs {
					match attr {
						OwnedTypeAttribute::Ident(i) => ident = Some(i),
						OwnedTypeAttribute::Derive(ds) => {
							for d in ds {
								derives.insert(d);
							}
						}
					}
				}

				match &mut self.owned {
					Some(sized) => {
						if let Some(i) = ident {
							sized.ident = i;
						}

						sized.derives.append(derives);
					}
					None => match ident {
						Some(ident) => self.owned = Some(OwnedTypeOptions { ident, derives }),
						None => {
							todo!()
						}
					},
				}
			}
			Attribute::Eq(types) => self.foreign.eq.extend(types),
			Attribute::Ord(types) => self.foreign.ord.extend(types),
			Attribute::Serde => self.foreign.serde = true,
			Attribute::NoDeref => self.no_deref = true,
			Attribute::Infallible => self.infallible = true,
		}

		Ok(())
	}
}

#[derive(Default)]
pub struct ForeignOptions {
	pub eq: Vec<syn::Type>,
	pub ord: Vec<syn::Type>,
	pub serde: bool,
}

pub struct OwnedTypeOptions {
	pub ident: syn::Ident,
	pub derives: Derives,
}

macro_rules! derives {
	($($field:ident: $variant:ident),*) => {
		pub enum Derive {
			$($variant),*
		}

		#[derive(Default)]
		pub struct Derives {
			$($field: bool),*
		}

		impl Derives {
			pub fn insert(&mut self, d: Derive) {
				match d {
					$(
						Derive::$variant => self.$field = true,
					)*
				}
			}

			pub fn append(&mut self, other: Self) {
				$(
					self.$field |= other.$field;
				)*
			}

			pub fn iter(&self) -> DerivesIter {
				DerivesIter {
					$(
						$field: self.$field,
					)*
				}
			}
		}

		impl<'a> IntoIterator for &'a Derives {
			type Item = Derive;
			type IntoIter = DerivesIter;

			fn into_iter(self) -> Self::IntoIter {
				self.iter()
			}
		}

		impl IntoIterator for Derives {
			type Item = Derive;
			type IntoIter = DerivesIter;

			fn into_iter(self) -> Self::IntoIter {
				self.iter()
			}
		}

		pub struct DerivesIter {
			$($field: bool),*
		}

		impl Iterator for DerivesIter {
			type Item = Derive;

			fn next(&mut self) -> Option<Self::Item> {
				$(
					if self.$field {
						self.$field = false;
						return Some(Derive::$variant)
					}
				)*

				None
			}
		}
	};
}

derives! {
	default: Default,
	partial_eq: PartialEq,
	eq: Eq,
	partial_ord: PartialOrd,
	ord: Ord,
	hash: Hash
}
