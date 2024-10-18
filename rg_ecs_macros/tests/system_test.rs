use rg_ecs_macros::system;

extern crate rg_ecs_macros;

#[test]
fn it_works() {
    system!(|_a:&i32, _b:&mut f64, _c: &String|{
        // todo - do something?
    });
}