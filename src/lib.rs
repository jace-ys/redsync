pub use crate::builder::RedlockBuilder;
pub use crate::redlock::{Lock, Redlock};

mod builder;
mod errors;
mod redlock;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
