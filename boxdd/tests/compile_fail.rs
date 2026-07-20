#[test]
fn safe_ownership_boundaries_do_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/live_id_from_raw.rs");
    cases.compile_fail("tests/ui/raw_id_from_ffi.rs");
    cases.compile_fail("tests/ui/owner_state_send.rs");
    cases.compile_fail("tests/ui/callback_world_removed.rs");
    cases.compile_fail("tests/ui/event_view_escape.rs");
    cases.compile_fail("tests/ui/user_data_borrow_escape.rs");

    #[cfg(feature = "serde")]
    cases.compile_fail("tests/ui/live_id_deserialize.rs");
}
