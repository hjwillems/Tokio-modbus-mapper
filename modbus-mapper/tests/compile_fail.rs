//! Compile-fail tests for the `#[derive(ModbusMapper)]` validation rules.
//!
//! These lock in that invalid mappings are rejected at compile time with a clear
//! message, rather than silently producing an incorrect register layout.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
