pub use str_newtype_derive::StrNewType;

pub unsafe trait Buffer: Sized {
	fn as_bytes(&self) -> &[u8];

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
