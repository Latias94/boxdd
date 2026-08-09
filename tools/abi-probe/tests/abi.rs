use std::collections::BTreeSet;

use boxdd_abi_probe as probe;

#[test]
fn c_and_rust_calls_report_the_expected_box2d_version() {
    const EXPECTED: (i32, i32, i32) = (3, 2, 0);

    assert_eq!(probe::c_version(), EXPECTED);
    assert_eq!(probe::rust_version(), EXPECTED);
}

#[test]
fn every_callback_typedef_crosses_c_and_rust_with_exact_sentinels() {
    const EXPECTED_CALLBACKS: [&str; 17] = [
        "b2AllocFcn",
        "b2AssertFcn",
        "b2CastResultFcn",
        "b2CustomFilterFcn",
        "b2EnqueueTaskCallback",
        "b2FinishTaskCallback",
        "b2FreeFcn",
        "b2FrictionCallback",
        "b2LogFcn",
        "b2OverlapResultFcn",
        "b2PlaneResultFcn",
        "b2PreSolveFcn",
        "b2RestitutionCallback",
        "b2TaskCallback",
        "b2TreeBoxCastCallbackFcn",
        "b2TreeQueryCallbackFcn",
        "b2TreeRayCastCallbackFcn",
    ];

    let callback_names = probe::callback_names();
    assert_eq!(callback_names.len(), EXPECTED_CALLBACKS.len());
    assert_eq!(
        callback_names.iter().copied().collect::<BTreeSet<_>>(),
        EXPECTED_CALLBACKS.into_iter().collect::<BTreeSet<_>>()
    );

    let results = probe::callback_probe_results();
    assert_eq!(results.len(), callback_names.len());
    for (expected_name, result) in callback_names.iter().zip(&results) {
        assert_eq!(result.name, *expected_name);
        assert_eq!(result.call_count, 1, "{} call count", result.name);
        assert_eq!(
            result.argument_match_count, 1,
            "{} sentinel arguments",
            result.name
        );
        assert!(result.return_matched, "{} sentinel return", result.name);
        let expected_nested_calls = u32::from(result.name == "b2EnqueueTaskCallback");
        assert_eq!(
            result.nested_call_count, expected_nested_calls,
            "{} nested callback count",
            result.name
        );
    }
}

#[test]
fn linked_library_matches_the_selected_precision_and_rejects_the_opposite_header_mode() {
    assert_eq!(probe::is_double_precision(), boxdd_sys::IS_DOUBLE_PRECISION);
    assert!(probe::precision_matches());
    assert!(!probe::mixed_precision_matches());
}

#[test]
fn tree_node_anonymous_unions_match_the_c_layout() {
    assert!(probe::tree_node_anonymous_union_layout_matches());
}
