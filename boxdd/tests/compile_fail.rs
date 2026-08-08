#[test]
fn safe_ownership_boundaries_do_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/live_id_from_raw.rs");
    cases.compile_fail("tests/ui/tree_proxy_id_from_raw.rs");
    cases.compile_fail("tests/ui/callback_world_removed.rs");
    cases.compile_fail("tests/ui/body_capability_conflicts_with_world_mutation.rs");
    cases.compile_fail("tests/ui/shape_capability_conflicts_with_world_mutation.rs");
    cases.compile_fail("tests/ui/chain_capability_conflicts_with_world_mutation.rs");
    cases.compile_fail("tests/ui/joint_capability_conflicts_with_world_mutation.rs");
    cases.compile_fail("tests/ui/query_capability_conflicts_with_world_mutation.rs");
    cases.compile_fail("tests/ui/recording_conflicts_with_preexisting_capability.rs");
    cases.compile_fail("tests/ui/recording_session_conflicts_with_custom_filter.rs");
    cases.compile_fail("tests/ui/recording_session_conflicts_with_pre_solve.rs");
    cases.compile_fail("tests/ui/body_capability_escape.rs");
    cases.compile_fail("tests/ui/typed_joint_family_boundary.rs");
    cases.compile_fail("tests/ui/context_free_definition_issuance.rs");
    cases.compile_fail("tests/ui/event_view_escape.rs");
    cases.compile_fail("tests/ui/replay_view_escape.rs");
    cases.compile_fail("tests/ui/user_data_borrow_escape.rs");

    #[cfg(feature = "serde")]
    cases.compile_fail("tests/ui/live_id_deserialize.rs");
}
