use nan_tag::*;

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
fn lifetime_miscompile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/lifetime-fail/fail.rs");
}
