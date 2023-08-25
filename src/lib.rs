#![no_std]
#![feature(strict_provenance)]

#[cfg(all(not(target_pointer_width = "64"), not(target_pointer_with = "128")))]
compile_error!("Pointer size must be 64 bits");

#[cfg(feature = "boxed_ptr")]
extern crate alloc;

use core::marker::PhantomData;
use core::ptr;

const POINTER_MASK: usize = 0x7FF8_0000_0000_0000;
const ACTUAL_NAN_BITS: usize = 0x7FFC_0000_0000_0000;
const ACTUAL_NAN: f64 = unsafe {
    core::mem::transmute::<usize, f64>(ACTUAL_NAN_BITS)
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ExtractedNan<'a, T: 'a> {
    Float(f64),
    Pointer(&'a T),
}

#[derive(Debug, PartialEq)]
pub enum ExtractedNanMut<'a, T: 'a> {
    Float(f64),
    Pointer(&'a mut T),
}

trait TaggedPtr<'a, T: 'a> {
    fn extract(&self) -> ExtractedNan<'a, T>;
    fn is_pointer(&self) -> bool {
        match self.extract() {
            ExtractedNan::Pointer(_) => true,
            _ => false,
        }
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

trait TaggedPtrMut<'a, T: 'a>: TaggedPtr<'a, T> {
    fn extract_mut(&mut self) -> ExtractedNanMut<'a, T>;
    fn as_mut(&mut self) -> Option<&'a mut T> {
        match self.extract_mut() {
            ExtractedNanMut::Pointer(ptr) => Some(ptr),
            _ => None,
        }
    }
}

// stores a pointer instead of an `f64` due to provenance
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TaggedNan<'a, T: 'a> {
    value: *const T,
    phantom: PhantomData<&'a T>,
}

impl TaggedNan<'_, ()> {
    pub fn new_float(value: f64) -> Self {
        Self::new_float_with(value)
    }
}
impl<'a, T> TaggedNan<'a, T> {
    pub fn new_float_with(value: f64) -> Self {
        let float = if value.is_nan() { ACTUAL_NAN } else { value };
        let bits = float.to_bits() as usize;
        TaggedNan {
            value: ptr::null::<T>().with_addr(bits),
            phantom: PhantomData,
        }
    }
    pub fn new_pointer(ptr: &'a T) -> Self {
        let ptr: *const T = &*ptr;
        TaggedNan {
            value: ptr.map_addr(|a| a ^ POINTER_MASK),
            phantom: PhantomData,
        }
    }
}
impl<'a, T> TaggedPtr<'a, T> for TaggedNan<'a, T> {
    fn extract(&self) -> ExtractedNan<'a, T> {
        let bits = self.value.addr();
        let float = f64::from_bits(bits as u64);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNan::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNan::Pointer(unsafe { ptr.as_ref::<'a>().unwrap() })
        }
    }
    fn is_pointer(&self) -> bool {
        self.value.addr() & POINTER_MASK == POINTER_MASK
    /*
        let bits = self.value.addr();
        let float = f64::from_bits(bits as u64);
        float.is_nan() && bits != ACTUAL_NAN_BITS
    */
    }
}

#[cfg(feature = "boxed_ptr")]
use alloc::boxed::Box;
#[cfg(feature = "boxed_ptr")]
#[derive(Debug, PartialEq)]
pub struct BoxedTaggedNan<T> {
    value: *mut T,
}

#[cfg(feature = "boxed_ptr")]
impl BoxedTaggedNan<()> {
    pub fn new_float(value: f64) -> Self {
        Self::new_float_with(value)
    }
}
#[cfg(feature = "boxed_ptr")]
impl<T> BoxedTaggedNan<T> {
    pub fn new_float_with(value: f64) -> Self {
        let float = if value.is_nan() { ACTUAL_NAN } else { value };
        let bits = float.to_bits() as usize;
        BoxedTaggedNan {
            value: ptr::null_mut::<T>().with_addr(bits),
        }
    }
    pub fn new_pointer(val: T) -> Self {
        let boxed = Box::new(val);
        let ptr = Box::into_raw(boxed);
        BoxedTaggedNan {
            value: ptr.map_addr(|a| a ^ POINTER_MASK),
        }
    }
}
#[cfg(feature = "boxed_ptr")]
impl<'a, T: 'a> TaggedPtr<'a, T> for BoxedTaggedNan<T> {
    fn extract(&self) -> ExtractedNan<'a, T> {
        let bits = self.value.addr();
        let float = f64::from_bits(bits as u64);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNan::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNan::Pointer(unsafe { ptr.as_ref::<'a>().unwrap() })
        }
    }
    fn is_pointer(&self) -> bool {
        let bits = self.value.addr();
        let float = f64::from_bits(bits as u64);
        float.is_nan() && bits != ACTUAL_NAN_BITS
    }
}
#[cfg(feature = "boxed_ptr")]
impl<'a, T: 'a> TaggedPtrMut<'a, T> for BoxedTaggedNan<T> {
    fn extract_mut(&mut self) -> ExtractedNanMut<'a, T> {
        let bits = self.value.addr();
        let float = f64::from_bits(bits as u64);
        if !float.is_nan() || bits == ACTUAL_NAN_BITS {
            ExtractedNanMut::Float(float)
        } else {
            let ptr = self.value.map_addr(|a| a ^ POINTER_MASK);
            ExtractedNanMut::Pointer(unsafe { ptr.as_mut::<'a>().unwrap() })
        }
    }
}
#[cfg(feature = "boxed_ptr")]
impl<T> Drop for BoxedTaggedNan<T> {
    fn drop(&mut self) {
        if self.is_pointer() {
            unsafe { Box::from_raw(self.value.map_addr(|a| a ^ POINTER_MASK)) };
        }
    }
}
#[cfg(feature = "boxed_ptr")]
impl<T: Clone> Clone for BoxedTaggedNan<T> {
    fn clone(&self) -> Self {
        if self.is_pointer() {
            let this_box = unsafe { Box::from_raw(self.value.map_addr(|a| a ^ POINTER_MASK ))};
            let new_ptr = Box::into_raw(this_box.clone());
            Box::leak(this_box);
            BoxedTaggedNan {
                value: new_ptr,
            }
        } else {
            BoxedTaggedNan {
                value: self.value,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_pointer() {
        let x = 9;
        let y = &x;
        let tagged = TaggedNan::new_pointer(y);
        assert_eq!(Some(y), tagged.as_ref());
    }

    #[test]
    fn extract_nan() {
        let tagged = TaggedNan::new_float(f64::NAN);
        if let Some(float) = tagged.as_float() {
            assert!(float.is_nan());
        } else {
            panic!("Failed to extract NaN");
        }
    }

    #[test]
    fn extract_float() {
        let tagged = TaggedNan::new_float(24.5);
        assert_eq!(Some(24.5), tagged.as_float());
    }

    #[test]
    fn reassigning() {
        let mut tagged = TaggedNan::<i32>::new_float_with(17.5);
        assert_eq!(Some(17.5), tagged.as_float());
        let data = 12;
        let ptr = &data;
        tagged = TaggedNan::new_pointer(&ptr);
        assert_eq!(Some(ptr), tagged.as_ref());
    }

    #[test]
    fn lifetime_float() {
        let tagged;
        {
            tagged = TaggedNan::new_float(20.5);
        }
        assert_eq!(Some(20.5), tagged.as_float());
    }

    // This test should fail to compile due to lifetimes
    #[test]
    #[cfg(cfail)]
    fn lifetime_miscompile() {
        let tagged;
        {
            let x = 17;
            let y = &x;
            tagged = TaggedNan::new_pointer(y);
        }
        tagged.extract();
    }

    #[cfg(feature = "boxed_ptr")]
    #[test]
    fn mutable_extract() {
        let x = alloc::string::String::from("hello");
        let mut boxed = BoxedTaggedNan::new_pointer(x);
        boxed.as_mut().unwrap().push_str(" world");
        assert_eq!("hello world", boxed.as_ref().unwrap().as_str());
    }

    #[cfg(feature = "boxed_ptr")]
    #[test]
    fn does_drop() {
        let x = alloc::rc::Rc::new(12);
        let copy = x.clone();
        let boxed = BoxedTaggedNan::new_pointer(copy);
        assert_eq!(2, alloc::rc::Rc::strong_count(&x));
        core::mem::drop(boxed);
        assert_eq!(1, alloc::rc::Rc::strong_count(&x));
    }
}
