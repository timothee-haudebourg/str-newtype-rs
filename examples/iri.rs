use static_automata::{Validate, grammar};
use str_newtype::StrNewType;

#[grammar(file = "iri.abnf", export("IRI"))]
mod automata {}

/// IRI.
#[derive(Validate, StrNewType, PartialEq, Eq, PartialOrd, Ord)]
#[automaton(automata::Iri)]
#[newtype(
    ord(str, &str, String),
    owned(IriBuf, derive(PartialEq))
)]
pub struct Iri(str);

fn main() {
	Iri::new("https://www.rust-lang.org/foo/bar?query#frag").unwrap();
}
