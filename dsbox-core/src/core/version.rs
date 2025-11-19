pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
