pub fn current() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
