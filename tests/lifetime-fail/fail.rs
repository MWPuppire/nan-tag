use nan_tag::*;

fn main() {
    let tagged;
    {
        let x = 17;
        let y = &x;
        tagged = TaggedNan::new_pointer(y);
    }
    tagged.extract();
}
