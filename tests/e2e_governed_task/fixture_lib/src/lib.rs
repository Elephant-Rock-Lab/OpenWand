/// Fixture library for Wave 05 end-to-end governed task tests.

/// Returns a greeting for the given name.
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_works() {
        assert_eq!("Hello, world!", hello("world"));
    }
}
