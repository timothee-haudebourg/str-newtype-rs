//! This library provides the handy `RegularGrammar` derive macro that helps you
//! create unsized type wrapping byte or char strings validated by a regular
//! grammar. It works by parsing a grammar specified in a file or the
//! documentation of your type, statically compiling it into a deterministic,
//! minimal, regular automaton then translated into a Rust validation function.
//!
//! For now, only the [ABNF] grammar format is supported.
//!
//! [ABNF]: <https://datatracker.ietf.org/doc/html/rfc5234>
//!
//! # Basic Usage
//!
//! The grammar is specified by code blocks in the type documentation.
//! The type itself must be a simple tutple struct with a single unnamed field
//! specifying the grammar "token string type". This token string type can be:
//! - `[u8]`: the grammar is defined on bytes.
//! - `str`: the grammar is defined on unicode characters.
//!
//! ## Example
//!
//! ```
//! use static_regular_grammar::RegularGrammar;
//!
//! /// Example grammar.
//! ///
//! /// ```abnf
//! /// foo = "f" 1*("oo") ; the first non-terminal is used as entry point.
//! /// ```
//! #[derive(RegularGrammar)]
//! pub struct Foo([u8]);
//!
//! let foo = Foo::new(b"foooooo").unwrap();
//! ```
//!
//! The derive macro also provides a `grammar` attribute to configure the
//! grammar and the generated code. With this attribute, instead of using the
//! documentation, you can specify a path to a file containing the grammar:
//!
//! ```
//! # use static_regular_grammar::RegularGrammar;
//! /// Example grammar.
//! #[derive(RegularGrammar)]
//! #[grammar(file = "examples/test.abnf")]
//! pub struct Foo([u8]);
//!
//! let foo = Foo::new(b"foooooo").unwrap();
//! ```
//!
//! # Grammar Entry Point
//!
//! By default the first non-terminal defined in the grammar is used as entry
//! point. You can specify a different entry point using the `entry_point`
//! sub-attribute of the `grammar` attribute:
//!
//! ```
//! # use static_regular_grammar::RegularGrammar;
//! /// Example grammar.
//! #[derive(RegularGrammar)]
//! #[grammar(file = "examples/test.abnf", entry_point = "bar")]
//! pub struct Bar([u8]);
//!
//! let bar = Bar::new(b"baaaar").unwrap();
//! ```
//!
//! # ASCII
//!
//! Using the `[u8]` token string type, it is possible to specify that the
//! value can be interpreted as an ASCII text string. Then the resulting type
//! will implement `Display`, `Deref<Target=str>`, `AsRef<str>`, ect.
//! ```
//! # use static_regular_grammar::RegularGrammar;
//! #[derive(RegularGrammar)]
//! #[grammar(file = "examples/test.abnf", ascii)]
//! pub struct Bar([u8]);
//!
//! let bar = Bar::new(b"baaaar").unwrap();
//! println!("{bar}");
//! ```
//!
//! # Sized Type
//!
//! The `RegularGrammar` macro works on unsized type, but it is often useful
//! to have an sized equivalent that can own the data while still guaranteeing
//! the validity of the data. The derive macro can do that for you using the
//! `sized` sub-attribute of the `grammar` attribute.
//!
//! ```
//! # use static_regular_grammar::RegularGrammar;
//! /// Example grammar, with sized variant.
//! ///
//! /// ```abnf
//! /// foo = "f" 1*("oo")
//! /// ```
//! #[derive(RegularGrammar)]
//! #[grammar(sized(FooBuf))] // this will generate a `FooBuf` type.
//! pub struct Foo([u8]);
//!
//! let foo = FooBuf::new(b"foooooo".to_vec()).unwrap();
//! ```
//!
//! The sized type will implement `Deref`, `Borrow` and `AsRef` to the unsized
//! type. It will also include a method named `as_unsized_type_name` (e.g.
//! `as_foo` in the example above) returning a reference to the unsized type.
//!
//! ## Common trait implementations
//!
//! You can specify what common trait to automatically implement for the sized
//! type using the `derive` sub-attribute.
//!
//! ```ignore
//! #[grammar(sized(FooBuf, derive(PartialEq, Eq)))]
//! ```
//!
//! The supported traits are:
//! - `Debug`
//! - `Display`
//! - `PartialEq`
//! - `Eq`
//! - `PartialOrd`
//! - `Ord`
//! - `Hash`
//!
//! All will rely on an equivalent implementation for the unsized type.
//!
//! # Caching
//!
//! When compiled, the input grammar is determinized and minimized. Those are
//! expensive operation that can take several seconds on large grammars.
//! To avoid unnecessary work, the resulting automaton is stored on disk until
//! changes are made to the grammar. By default, the automaton will be stored
//! in the `target` folder, as `regular-grammar/TypeName.automaton.cbor`. For
//! instance, in the example above the path will be
//! `target/regular-grammar/Foo.automaton.cbor`.
//! You can specify the file path yourself using the `cache` sub-attribute:
//!
//! ```ignore
//! #[grammar(cache = "path/to/cache.automaton.cbor")]
//! ```
//!
//! The path must be relative, and must not include `..` segments.
//! If you have multiple grammar types having the same name, use this attribute
//! to avoid conflicts, otherwise caching will not work.
//! For large grammars, it might be a good idea to cache the automaton directly
//! with the sources, and ship it with your library/application to reduce
//! compilation time on the user machine.
//!
//! # Disable automaton generation
//!
//! When using a linter such as [`rust-analyzer`], it may be too expensive to
//! regenerate the grammar automaton continually, even with caching. On large
//! grammars the generated automaton code can span hundreds or even thousands
//! of lines. In that case it is possible to disable the automaton generation
//! all together using the `disable` option:
//! ```ignore
//! #[grammar(disable)]
//! ```
//!
//! Of course it is best to use this option behind a feature used only by the
//! linter:
//! ```ignore
//! #[cfg_attr(feature = "disable-grammars", grammar(disable))]
//! ```
//!
//! [`rust-analyzer`](https://rust-analyzer.github.io/)
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use syn::parse_macro_input;

mod utils;
mod attribute;
mod options;
mod new_type;

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
	Syn(#[from] syn::Error)
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
			Self::Syn(e) => e.span()
		}
	}
}

#[proc_macro_derive(StrNewType, attributes(newtype))]
#[proc_macro_error]
pub fn derive_regular_grammar(input_tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input_tokens as syn::DeriveInput);
	match new_type::derive(input) {
		Ok(tokens) => tokens.into(),
		Err(e) => {
			let span = e.span();
			abort!(span, e)
		},
	}
}