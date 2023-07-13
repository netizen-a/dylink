// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;
impl<'a, L: Loader + 'a> LibraryLock<'a> for Library<'a, L> {
	type Guard = LibraryGuard<'a, L>;

	fn lock(&'a self) -> LockResult<Self::Guard> {
		self.hlib
			.lock()
			.map(|guard| LibraryGuard {
				libs: self.libs,
				guard,
			})
			.or_else(|poison| {
				Err(PoisonError::new(LibraryGuard {
					libs: self.libs,
					guard: poison.into_inner(),
				}))
			})
	}
}

impl<'a, L: Close + 'a> LibraryLock<'a> for CloseableLibrary<'a, L> {
	type Guard = CloseableLibraryGuard<'a, L>;

	fn lock(&'a self) -> LockResult<Self::Guard> {
		self.inner
			.lock()
			.map(|guard| CloseableLibraryGuard {
				libs: self.libs,
				guard,
			})
			.or_else(|poison| {
				Err(PoisonError::new(CloseableLibraryGuard {
					libs: self.libs,
					guard: poison.into_inner(),
				}))
			})
	}
}
