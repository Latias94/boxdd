fn assert_sha256(label: &str, digest: &str) {
    assert_eq!(digest.len(), 64, "{label} must be a SHA-256 digest");
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "{label} must use lowercase hexadecimal"
    );
}

fn required_env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|error| panic!("{name} must be supplied at runtime: {error}"))
}

unsafe extern "C" fn count_overlap(
    shape_id: boxdd_sys::ffi::b2ShapeId,
    context: *mut std::ffi::c_void,
) -> bool {
    if unsafe { boxdd_sys::ffi::b2Shape_IsValid(shape_id) } {
        let count = unsafe { &mut *context.cast::<usize>() };
        *count += 1;
    }
    true
}

fn exercise_world_lifecycle() {
    unsafe {
        let world_def = boxdd_sys::ffi::b2DefaultWorldDef();
        let world = boxdd_sys::ffi::b2CreateWorld(&world_def);
        assert!(boxdd_sys::ffi::b2World_IsValid(world));

        let mut body_def = boxdd_sys::ffi::b2DefaultBodyDef();
        body_def.type_ = boxdd_sys::ffi::b2BodyType_b2_dynamicBody;
        let body = boxdd_sys::ffi::b2CreateBody(world, &body_def);
        let shape_def = boxdd_sys::ffi::b2DefaultShapeDef();
        let polygon = boxdd_sys::ffi::b2MakeBox(0.5, 0.5);
        let shape = boxdd_sys::ffi::b2CreatePolygonShape(body, &shape_def, &polygon);
        assert!(boxdd_sys::ffi::b2Shape_IsValid(shape));

        boxdd_sys::ffi::b2World_Step(world, 1.0 / 60.0, 4);
        let aabb = boxdd_sys::ffi::b2AABB {
            lowerBound: boxdd_sys::ffi::b2Vec2 { x: -2.0, y: -2.0 },
            upperBound: boxdd_sys::ffi::b2Vec2 { x: 2.0, y: 2.0 },
        };
        let origin = boxdd_sys::ffi::b2Pos { x: 0.0, y: 0.0 };
        let filter = boxdd_sys::ffi::b2DefaultQueryFilter();
        let mut overlap_count = 0_usize;
        boxdd_sys::ffi::b2World_OverlapAABB(
            world,
            origin,
            aabb,
            filter,
            Some(count_overlap),
            std::ptr::from_mut(&mut overlap_count).cast(),
        );
        assert!(overlap_count >= 1);

        boxdd_sys::ffi::b2DestroyWorld(world);
        assert!(!boxdd_sys::ffi::b2World_IsValid(world));
    }
}

fn main() {
    let expected_provider = required_env("BOXDD_NATIVE_QUALIFICATION_PROVIDER");
    let expected_manifest = required_env("BOXDD_NATIVE_QUALIFICATION_MANIFEST_SHA256");
    let expected_archive = required_env("BOXDD_NATIVE_QUALIFICATION_ARCHIVE_SHA256");
    let expected_provenance = required_env("BOXDD_NATIVE_QUALIFICATION_PROVENANCE_SHA256");
    let expected_trusted_root = required_env("BOXDD_NATIVE_QUALIFICATION_TRUSTED_ROOT_SHA256");
    assert!(matches!(expected_provider.as_str(), "system" | "prebuilt"));
    assert_eq!(boxdd_sys::PROVIDER_ADAPTER, expected_provider);
    assert_sha256("provider manifest", &expected_manifest);
    assert_sha256("provider archive", &expected_archive);
    assert_eq!(boxdd_sys::PROVIDER_MANIFEST_SHA256, expected_manifest);
    assert_eq!(boxdd_sys::PROVIDER_ARCHIVE_SHA256, expected_archive);
    match expected_provider.as_str() {
        "system" => {
            assert!(expected_provenance.is_empty());
            assert!(expected_trusted_root.is_empty());
            assert!(boxdd_sys::PROVIDER_PROVENANCE_SHA256.is_empty());
            assert!(boxdd_sys::PROVIDER_TRUSTED_ROOT_SHA256.is_empty());
        }
        "prebuilt" => {
            assert_sha256("provider provenance", &expected_provenance);
            assert_sha256("provider trusted root", &expected_trusted_root);
            assert_eq!(boxdd_sys::PROVIDER_PROVENANCE_SHA256, expected_provenance);
            assert_eq!(
                boxdd_sys::PROVIDER_TRUSTED_ROOT_SHA256,
                expected_trusted_root
            );
        }
        _ => unreachable!(),
    }

    let identity = boxdd_sys::adapter::verify_runtime_identity()
        .expect("qualified provider identity must match this exact Rust crate build");
    assert_ne!(identity.snapshot_layout_hash, 0);
    assert!(!unsafe { boxdd_sys::adapter::boxddRecPlayer_IsHealthy(std::ptr::null()) });
    assert!(
        boxdd_sys::adapter::validate_snapshot(&[], &boxdd_sys::adapter::SnapshotLimits::default(),)
            .is_err()
    );

    let version = unsafe { boxdd_sys::ffi::b2GetVersion() };
    assert_eq!(version.major, 3);
    assert_eq!(version.minor, 2);
    exercise_world_lifecycle();
    let receipt = required_env("BOXDD_NATIVE_QUALIFICATION_RECEIPT");
    let nonce = required_env("BOXDD_NATIVE_QUALIFICATION_NONCE");
    std::fs::write(receipt, nonce)
        .expect("qualified consumer must write its exact execution receipt");
    println!(
        "qualified native Box2D {}.{}.{} via {} ({})",
        version.major,
        version.minor,
        version.revision,
        boxdd_sys::PROVIDER_ADAPTER,
        boxdd_sys::ABI_PRECISION.as_str()
    );
}
