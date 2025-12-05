# Safe wrappers around string types made easy

[![Build](https://img.shields.io/github/actions/workflow/status/timothee-haudebourg/str-newtype-rs/ci.yml?branch=main&style=flat-square)](https://github.com/timothee-haudebourg/str-newtype-rs/actions)
[![Crate informations](https://img.shields.io/crates/v/str-newtype.svg?style=flat-square)](https://crates.io/crates/str-newtype)
[![License](https://img.shields.io/crates/l/str-newtype.svg?style=flat-square)](https://github.com/timothee-haudebourg/str-newtype-rs#license)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/str-newtype)

<!-- cargo-rdme start -->

This library provides a handy derive macro to create type-safe wrappers
around borrowed and owned string types (`str` and `String`), guaranteeing
through the type system that those string have been validated.

With this derive macro, all you need to do is define the base borrowed type,
specify the name of the associated owned type and what trait you want them
to implement.

## Example

In this example, we wish to define the types `FooStr` and `FooString`,
similar to `str` (borrowed) and `String` (owned), with the extra guarantee
that the string is always equal to `"foo"`.

```rust
use str_newtype::StrNewType;

/// An `str` that is equal to `"foo"`.
#[derive(StrNewType)]
#[newtype(owned(FooString))]
pub struct FooStr(str);

impl FooStr {
  pub const fn validate_bytes(s: &[u8]) -> bool {
    s.len() == 3 && s[0] == b'f' && s[1] == b'f' && s[2] == b'f'
  }

  pub const fn validate_str(s: &str) -> bool {
    Self::validate_bytes(s.as_bytes())
  }
}
```

The validation methods (`validate_*`) are provided by us, but `StrNewType`
will use them to derive the following:
- An error type `InvalidFooStr<T = String>(pub T)`;
- A constructor `FooStr::new<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<&Self, InvalidFooStr<&T>>`
- `FooStr: Deref<Target = str>` implementation
- `FooStr: AsRef<str>` implementation
- `FooStr: Borrow<str>` implementation

Since we added the `owned(FooString)` attribute, it will also generate:
- A sized/owned version of `FooStr`: `struct FooString(String);`
- A constructor `FooString::new<T: Buffer>(s: T) -> Result<Self, InvalidFooStr<T>>`
- `FooString: Deref<Target = FooStr>`
- `FooString: AsRef<FooStr>` implementation
- `FooString: Borrow<FooStr>` implementation

And much more. See the the [`StrNewType`] documentation for a full
specification of what items are derived and how it can be controlled with
the `newtype` attribute.

<!-- cargo-rdme end -->

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
