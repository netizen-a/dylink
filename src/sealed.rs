#[allow(dead_code)]
pub trait Sealed {}

#[repr(C)]
pub struct Opaque {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl Sealed for Opaque {}
