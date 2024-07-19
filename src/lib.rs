#![allow(unstable_name_collisions)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(strict_provenance))]

#[cfg(any(
    target_pointer_width = "32",
    target_pointer_width = "16"
))]
compile_error!("Pointer size must be at least 64-bits");

#[cfg(not(feature = "nightly"))]
extern crate sptr;

use core::marker::PhantomData;
use core::ptr;
#[cfg(not(feature = "nightly"))]
use sptr::Strict;

const POINTER_MASK: usize = 0x7FF8_0000_0000_0000;
const ACTUAL_NAN_BITS: u64 = 0x7FFC_0000_0000_0000;
// `const from_bits` isn't stable yet, so an unsafe transmute is used.
const ACTUAL_NAN: f64 = unsafe { core::mem::transmute::<u64, f64>(ACTUAL_NAN_BITS) };

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ExtractedNan<'a, T: 'a> {
    Float(f64),
    Pointer(&'a T),
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq)]
pub enum ExtractedNanMut<'a, T: 'a> {
    Float(f64),
    PointerMut(&'a mut T),
}

pub trait TaggedPtr<'a, T: 'a> {
    fn extract(&self) -> ExtractedNan<'a, T>;

    fn is_pointer(&self) -> bool {
        matches!(self.extract(), ExtractedNan::Pointer(_))
    }
    fn as_float(&self) -> Option<f64> {
        match self.extract() {
            ExtractedNan::Float(float) => Some(float),
            _ => None,
        }
    }
    fn as_ref(&self) -> Option<&'a T> {
        match self.extract() {
            ExtractedNan::Pointer(ptr) => Some(ptr),
            _ => None,
        }
    }
}

pub trait TaggedPtrMut<'a, T: 'a>: TaggedPtr<'a, T> {
    fn extract_mut(&mut self) -> ExtractedNanMut<'a, T>;

    fn as_mut(&mut self) -> Option<&'a mut T> {
        match self.extract_mut() {
            ExtractedNanMut::PointerMut(ptr) => Some(ptr),
            _ => None,
        }
    }
}

/// A NaN-tagged pointer to a non-owned value or a 64-bit floating-point,
/// discriminated by NaN tag.
#[derive(Copy, Clone, Debug)]
pub struct TaggedNan<'a, T: 'a> {
    value: *const T,
    phantom: PhantomData<&'a T>,
}

/// A NaN-tagged pointer to an owned value or a 64-bit floating-point,
/// discriminated by NaN tag.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct BoxedTaggedNan<T> {
    value: *mut T,
    phantom: PhantomData<T>,
}

impl TaggedNan<'_, ()> {
    pub fn new_float(value: f64) -> Self {
        Self::new_float_with(value)
    }
}

impl<'a, T: 'a> TaggedNan<'a, T> {
    pub fn new_float_with(value: f64) -> Self {
        let float = if value.is_nan() { ACTUAL_NAN } else { value };
        let bits = float.to_bits() as usize;
        TaggedNan {
            value: ptr::null::<T>().with_addr(bits),
            phantom: PhantomData,
        }
    }

    pub fn new_pointer(ptr: &'a T) -> Self {
        let ptr: *const T = ptr;
        TaggedNan {
            value: ptr.map_addr(|a| a ^ POINTER_MASK),
            phantom: PhantomData,
        }
    }
}

impl<'a, T: 'a> TaggedPtr<'a, T> for TaggedNan<'a, T> {
    fn extract(&self) -> ExtractedNan<'a, T> {
        let bits = self.value.addr() as u64;
        let float = f64::from_bits(bits);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNan::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNan::Pointer(unsafe { ptr.as_ref::<'a>().unwrap() })
        }
    }
}

impl<'a, T: PartialEq + 'a> PartialEq for TaggedNan<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        if self.is_pointer() {
            unsafe { *self.value == *other.value }
        } else {
            let self_bits = self.value.addr() as u64;
            let other_bits = other.value.addr() as u64;
            f64::from_bits(self_bits) == f64::from_bits(other_bits)
        }
    }
}

#[cfg(feature = "std")]
impl BoxedTaggedNan<()> {
    pub fn new_float(value: f64) -> Self {
        Self::new_float_with(value)
    }
}

#[cfg(feature = "std")]
impl<T> BoxedTaggedNan<T> {
    pub fn new_float_with(value: f64) -> Self {
        let float = if value.is_nan() { ACTUAL_NAN } else { value };
        let bits = float.to_bits() as usize;
        BoxedTaggedNan {
            value: ptr::null_mut::<T>().with_addr(bits),
            phantom: PhantomData,
        }
    }

    pub fn new_pointer(val: T) -> Self {
        let boxed = Box::new(val);
        let ptr = Box::into_raw(boxed);
        BoxedTaggedNan {
            value: ptr,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<'a, T: 'a> TaggedPtr<'a, T> for BoxedTaggedNan<T> {
    fn extract(&self) -> ExtractedNan<'a, T> {
        let bits = self.value.addr() as u64;
        let float = f64::from_bits(bits);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNan::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNan::Pointer(unsafe { ptr.as_ref::<'a>().unwrap() })
        }
    }
}

#[cfg(feature = "std")]
impl<'a, T: 'a> TaggedPtrMut<'a, T> for BoxedTaggedNan<T> {
    fn extract_mut(&mut self) -> ExtractedNanMut<'a, T> {
        let bits = self.value.addr() as u64;
        let float = f64::from_bits(bits);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNanMut::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNanMut::PointerMut(unsafe { ptr.as_mut::<'a>().unwrap() })
        }
    }
}

#[cfg(feature = "std")]
impl<T> Drop for BoxedTaggedNan<T> {
    fn drop(&mut self) {
        if self.is_pointer() {
            drop(unsafe { Box::from_raw(self.value) });
        }
    }
}

#[cfg(feature = "std")]
impl<T: Clone> Clone for BoxedTaggedNan<T> {
    fn clone(&self) -> Self {
        if self.is_pointer() {
            let this_box = unsafe { Box::from_raw(self.value) };
            let new_ptr = Box::into_raw(this_box.clone());
            Box::leak(this_box);
            BoxedTaggedNan {
                value: new_ptr,
                phantom: PhantomData,
            }
        } else {
            BoxedTaggedNan {
                value: self.value,
                phantom: PhantomData,
            }
        }
    }
}

#[cfg(feature = "std")]
impl<T: PartialEq> PartialEq for BoxedTaggedNan<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.is_pointer() {
            unsafe { *self.value == *other.value }
        } else {
            let self_bits = self.value.addr() as u64;
            let other_bits = other.value.addr() as u64;
            f64::from_bits(self_bits) == f64::from_bits(other_bits)
        }
    }
}
