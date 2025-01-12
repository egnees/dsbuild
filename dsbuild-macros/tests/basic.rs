use dsbuild_macros::Passable;
use dsbuild_message::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Passable, Clone, Copy)]
struct A {
    x: i32,
}

#[test]
fn basic() {
    let a = A { x: 1 };
    let b: Message = a.into();
    let ret: A = b.into();
    assert_eq!(ret.x, a.x);
}
