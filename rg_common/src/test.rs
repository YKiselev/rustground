
trait ToArg<'a, T>: Sized
where
    T: Sized + 'a,
{
    fn to_arg(arg: &str) -> Option<T>;
}

impl<'a> ToArg<'a, i32> for i32 {
    fn to_arg(arg: &str) -> Option<i32> {
        arg.parse().ok()
    }
}

impl<'a> ToArg<'a, &'a str> for &str {
    #[inline(always)]
    fn to_arg(arg: &str) -> Option<&'a str> {
        Some("aaa")
    }
}

fn p(value:&str) -> &str {
    value
}

fn register_handler<A, F>(handler: F) -> Box<dyn Fn(&str)>
where
    F: Fn(A) + 'static,
    A: for<'a> ToArg<'a, A>,
{
    let invoker = move |arg: &str| (handler)(A::to_arg(arg).unwrap());

    Box::new(invoker)
}

#[cfg(test)]
mod tests {

    use super::{register_handler};

    #[test]
    fn omg() {
        let mut handlers = Vec::new();
        handlers.push(register_handler(|a: i32| println!("Got: {a}")));
        //handlers.push(register_handler(|a: &str| println!("Got: {a}")));
        handlers.iter().for_each(|v| {
            (v)("123");
        });
    }
}
