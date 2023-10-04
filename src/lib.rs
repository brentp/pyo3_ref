use mlua::*;
use std::io;
use std::sync::{Arc, Mutex};

pub struct Inner {
    i: i32,
    j: i32,
}

pub struct Outer {
    inner: Arc<Mutex<Inner>>,
    a: i32,
    b: i32,
}

fn main() -> io::Result<()> {
    let lua = Lua::new();

    lua.register_userdata_type::<Inner>(|lp| {
        lp.add_field_method_get("i", |_, this| Ok(this.i));
        lp.add_field_method_get("j", |_, this| Ok(this.j));
    })
    .unwrap();

    lua.register_userdata_type::<Outer>(|lp| {
        lp.add_field_method_get("a", |_, this| Ok(this.a));
        lp.add_field_method_get("b", |_, this| Ok(this.b));
        lp.add_field_method_get("inner", |_, this| Ok(this.inner));
    })
    .unwrap();

    Ok(())
}
