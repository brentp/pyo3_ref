use mlua::*;
use parking_lot::Mutex;
use std::io;
use std::sync::Arc;

//use std::rc::Rc as Arc; // must be send.

pub struct Inner {
    i: i32,
    j: i32,
}

pub struct Outer {
    inner: Arc<Mutex<Inner>>,
    a: i32,
    b: i32,
}

//impl UserData for Inner {}
impl UserData for Outer {} // it seems this is required.

pub fn main() -> io::Result<()> {
    let lua = Lua::new();

    lua.register_userdata_type::<Arc<Mutex<Inner>>>(|lp| {
        lp.add_field_method_get("i", |_, this| Ok(this.lock().i));
        lp.add_field_method_get("j", |_, this| Ok(this.lock().j));
    })
    .unwrap();

    lua.register_userdata_type::<Outer>(|lp| {
        lp.add_field_method_get("a", |_, this| Ok(this.a));
        lp.add_field_method_get("b", |_, this| Ok(this.b));
        lp.add_field_method_get("inner", |lua, this| {
            lua.create_any_userdata(this.inner.clone())
        });
    })
    .unwrap();

    // make an example constructor and data. to test.
    let outer_constructor = lua
        .create_function(|_lua, (a, b): (i32, i32)| {
            Ok(Outer {
                inner: Arc::new(Mutex::new(Inner { i: a, j: b })),
                a: 2 * a,
                b: 2 * b,
            })
        })
        .unwrap();
    lua.globals().set("Outer", outer_constructor).unwrap();

    let code = r#"
        local outer = Outer(1, 2)
        print(outer.a, outer.b)
        print(outer.inner.i, outer.inner.j)
        "#;
    lua.load(code).exec().unwrap();

    Ok(())
}
