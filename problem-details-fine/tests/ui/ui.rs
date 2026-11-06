//! Compile-fail tests for the macros, one directory for each macro.
//!
//! The expected compiler output depends on the toolchain. To update it, run the tests with `TRYBUILD=overwrite`.

#[test]
fn problem_type() {
    trybuild::TestCases::new().compile_fail("tests/ui/problem_type/*.rs");
}

#[test]
fn problem_set() {
    trybuild::TestCases::new().compile_fail("tests/ui/problem_set/*.rs");
}
