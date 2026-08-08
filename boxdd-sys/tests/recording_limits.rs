use std::ffi::CString;

use boxdd_sys::ffi;

struct NativeRecordingFixture {
    world: ffi::b2WorldId,
    recording: *mut ffi::b2Recording,
}

impl Drop for NativeRecordingFixture {
    fn drop(&mut self) {
        // SAFETY: this fixture exclusively owns both handles. Stopping first also keeps teardown
        // valid if an assertion fires after native recording unexpectedly becomes attached.
        unsafe {
            ffi::b2World_StopRecording(self.world);
            ffi::b2DestroyRecording(self.recording);
            ffi::b2DestroyWorld(self.world);
        }
    }
}

#[test]
fn sticky_writer_failure_hides_bytes_and_rejects_file_save() {
    // SAFETY: definitions and paths remain alive for each call. The fixture exclusively owns the
    // returned handles and stops recording before destroying either one.
    unsafe {
        let world_def = ffi::b2DefaultWorldDef();
        let world = ffi::b2CreateWorld(&world_def);
        assert!(ffi::b2World_IsValid(world));

        let recording = ffi::b2CreateRecording(1);
        assert!(!recording.is_null());
        let fixture = NativeRecordingFixture { world, recording };

        ffi::b2World_StartRecording(fixture.world, fixture.recording);
        assert_eq!(ffi::b2Recording_GetSize(fixture.recording), -1);
        assert!(ffi::b2Recording_GetData(fixture.recording).is_null());

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("failed-recording.b2rec");
        let native_path = CString::new(path.to_string_lossy().as_bytes()).unwrap();
        assert!(!ffi::b2SaveRecordingToFile(
            fixture.recording,
            native_path.as_ptr()
        ));
        assert!(!path.exists());
    }
}

#[test]
fn rebuild_static_tree_records_the_manifest_opcode() {
    const RECORDING_HEADER_BYTES: usize = 32;
    const SNAPSHOT_SIZE_OFFSET: usize = 24;
    const RECORD_HEADER_BYTES: usize = 4;
    const REBUILD_STATIC_TREE_OPCODE: u8 = 0x0C;

    // SAFETY: the fixture exclusively owns the native handles. The returned recording bytes are
    // read only while the recording remains alive and detached from the world.
    unsafe {
        let world_def = ffi::b2DefaultWorldDef();
        let world = ffi::b2CreateWorld(&world_def);
        assert!(ffi::b2World_IsValid(world));

        let recording = ffi::b2CreateRecording(1024 * 1024);
        assert!(!recording.is_null());
        let fixture = NativeRecordingFixture { world, recording };

        ffi::b2World_StartRecording(fixture.world, fixture.recording);
        ffi::b2World_RebuildStaticTree(fixture.world);
        ffi::b2World_StopRecording(fixture.world);

        let size = usize::try_from(ffi::b2Recording_GetSize(fixture.recording)).unwrap();
        let data = ffi::b2Recording_GetData(fixture.recording);
        assert!(!data.is_null());
        let bytes = std::slice::from_raw_parts(data, size);
        assert!(bytes.len() >= RECORDING_HEADER_BYTES);

        let snapshot_size = u64::from_le_bytes(
            bytes[SNAPSHOT_SIZE_OFFSET..RECORDING_HEADER_BYTES]
                .try_into()
                .unwrap(),
        );
        let mut cursor = RECORDING_HEADER_BYTES + usize::try_from(snapshot_size).unwrap();
        let mut opcodes = Vec::new();
        while cursor < bytes.len() {
            assert!(bytes.len() - cursor >= RECORD_HEADER_BYTES);
            let opcode = bytes[cursor];
            let payload_size = usize::from(bytes[cursor + 1])
                | (usize::from(bytes[cursor + 2]) << 8)
                | (usize::from(bytes[cursor + 3]) << 16);
            cursor = cursor
                .checked_add(RECORD_HEADER_BYTES + payload_size)
                .expect("record cursor must not overflow");
            assert!(cursor <= bytes.len());
            opcodes.push(opcode);
        }

        assert!(
            opcodes.contains(&REBUILD_STATIC_TREE_OPCODE),
            "compiled b2World_RebuildStaticTree producer emitted {opcodes:?}"
        );
    }
}
