#[test]
fn native_identity_authority_cannot_be_forged_or_mutated() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/runtime_marker_tuple_construction.rs");
    cases.compile_fail("tests/ui/context_world_mut.rs");
    cases.compile_fail("tests/ui/context_resource_mut.rs");
}
