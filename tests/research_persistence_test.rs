//! Integration tests for research persistence: atomic writes, permanent advisory locks,
//! runtime directory security / socket limits, and path traversal rejection.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use emu::constants::research::storage::{MAX_DARWIN_SUN_PATH, SECURE_DIR_MODE, SECURE_FILE_MODE};
use emu::persistence::atomic::write_atomic;
use emu::persistence::lock::AdvisoryLock;
use emu::persistence::paths::{
    PathError, ResearchPaths, RuntimeDirectory, validate_path_component,
};

// ============================================================================
// 1. Atomic Staged Writes
// ============================================================================

#[tokio::test]
async fn test_atomic_write_creates_file_with_correct_content_and_permissions() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let target = temp.path().join("test_file.json");
    let data = b"{\"status\": \"initialized\", \"version\": 1}\n";

    write_atomic(&target, data)
        .await
        .expect("write_atomic failed");

    assert!(target.exists(), "Target file must exist after write_atomic");
    let read_back = std::fs::read(&target).expect("Failed to read back file");
    assert_eq!(read_back, data);

    #[cfg(unix)]
    {
        let meta = std::fs::metadata(&target).expect("Failed to stat target");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, SECURE_FILE_MODE,
            "File permissions must be 0600, got 0{mode:o}"
        );
    }
}

#[tokio::test]
async fn test_atomic_write_replaces_existing_content_cleanly() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let target = temp.path().join("record.json");

    let initial = b"{\"version\": 1, \"state\": \"draft\"}";
    write_atomic(&target, initial)
        .await
        .expect("initial write failed");
    assert_eq!(std::fs::read(&target).unwrap(), initial);

    let replacement = b"{\"version\": 2, \"state\": \"committed\"}";
    write_atomic(&target, replacement)
        .await
        .expect("replacement write failed");
    assert_eq!(std::fs::read(&target).unwrap(), replacement);

    // Verify no temporary .tmp files are left behind
    let mut entries = std::fs::read_dir(temp.path()).expect("read_dir failed");
    let count = entries.by_ref().count();
    assert_eq!(count, 1, "Only the target file should remain in directory");
}

#[tokio::test]
async fn test_atomic_write_concurrent_no_byte_mixing() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let target = temp.path().join("concurrent.dat");

    // Initialize with baseline content
    write_atomic(&target, b"initial")
        .await
        .expect("initial write failed");

    let mut handles = Vec::new();
    const NUM_WRITERS: usize = 16;
    const PAYLOAD_LEN: usize = 4096;

    for i in 0..NUM_WRITERS {
        let path = target.clone();
        let byte_val = b'A' + (i as u8 % 26);
        let payload = vec![byte_val; PAYLOAD_LEN];

        handles.push(tokio::spawn(
            async move { write_atomic(&path, &payload).await },
        ));
    }

    for handle in handles {
        let res = handle.await.expect("task join failed");
        assert!(res.is_ok(), "Concurrent write_atomic must succeed");
    }

    // Read back final content: must be exactly PAYLOAD_LEN of identical bytes (no mixing)
    let final_bytes = std::fs::read(&target).expect("Failed to read back file");
    assert_eq!(
        final_bytes.len(),
        PAYLOAD_LEN,
        "File length must match exact payload size"
    );

    let first_byte = final_bytes[0];
    assert!(
        final_bytes.iter().all(|&b| b == first_byte),
        "File must contain pure single-writer bytes, never mixed bytes from concurrent writes"
    );

    // Verify no stray .tmp files were orphaned
    let entries = std::fs::read_dir(temp.path()).expect("read_dir failed");
    let tmp_count = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
        .count();
    assert_eq!(tmp_count, 0, "No orphaned .tmp files should remain");
}

#[tokio::test]
async fn test_atomic_write_failure_preserves_existing_target_and_cleans_tmp() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");

    // Construct a valid target filename of 240 bytes (<= NAME_MAX of 255 on Unix)
    // Sibling staging filename: "{name}.{uuid}.tmp" -> 240 + 1 + 32 + 1 + 3 = 277 bytes > 255
    // Staging file creation will fail with ENAMETOOLONG without touching the pre-existing target!
    let long_basename = "a".repeat(240);
    let target = temp.path().join(long_basename);
    let original_data = b"CONFIDENTIAL_ORIGINAL_TARGET_BYTES";

    // Initial write using standard fs write to create the pre-existing target
    std::fs::write(&target, original_data).expect("Failed to write initial target");
    assert_eq!(std::fs::read(&target).unwrap(), original_data);

    // Attempt write_atomic on this existing target: staging file basename exceeds NAME_MAX
    let new_data = b"NEW_DATA_THAT_SHOULD_NEVER_OVERWRITE_TARGET";
    let err = write_atomic(&target, new_data)
        .await
        .expect_err("write_atomic with staging basename exceeding NAME_MAX must fail");
    assert!(
        err.to_string().contains("staging file"),
        "Error must describe staging failure: {err}"
    );

    // Assert pre-existing target was NOT truncated or modified
    let read_back = std::fs::read(&target).expect("Target must still exist");
    assert_eq!(
        read_back, original_data,
        "Original target file content must remain completely intact"
    );

    // Verify no orphaned .tmp files remain in the directory
    let entries = std::fs::read_dir(temp.path()).expect("read_dir failed");
    let tmp_count = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
        .count();
    assert_eq!(tmp_count, 0, "No .tmp staging files should be orphaned");
}

// ============================================================================
// 2. Cross-Process Advisory Locking & Permanent Lockfiles
// ============================================================================

/// Helper invoked by the spawned child process in cross-process contention tests.
/// Skips execution when run during standard cargo test discovery without the env var.
#[test]
fn run_child_lock_probe() {
    let lock_path_str = match std::env::var("EMU_TEST_LOCK_CHILD_PATH") {
        Ok(p) => p,
        Err(_) => return, // Running as standard test, skip probe mode
    };
    let lock_path = PathBuf::from(lock_path_str);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build current thread runtime for probe");

    rt.block_on(async {
        // Step 1: Parent holds the lock. Child attempt MUST fail with contention (exit code 5).
        let err = AdvisoryLock::try_acquire(&lock_path)
            .await
            .expect_err("Child probe must observe lock contention while parent holds lock");
        assert_eq!(err.exit_code(), 5, "Child probe must observe exit code 5");

        // Notify parent via stdout that child observed contention
        use std::io::Write;
        println!("CHILD_OBSERVED_CONTENTION");
        std::io::stdout().flush().unwrap();

        // Step 2: Wait for parent signal on stdin indicating lock has been released
        use std::io::Read;
        let mut buf = [0u8; 1];
        let _ = std::io::stdin().read_exact(&mut buf);

        // Step 3: Now acquisition MUST succeed
        let lock = AdvisoryLock::try_acquire(&lock_path)
            .await
            .expect("Child probe must acquire lock after parent release");
        println!("CHILD_ACQUIRED_LOCK");
        std::io::stdout().flush().unwrap();
        drop(lock);
    });
}

#[tokio::test]
async fn test_advisory_lock_cross_process_contention() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let lock_path = temp
        .path()
        .join("instances")
        .join("locks")
        .join("vm_cross_proc.run.lock");

    // Parent acquires lock
    let parent_lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Parent failed to acquire lock");

    // Spawn child probe process using current_exe
    use std::io::{BufRead, BufReader, Write};
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("run_child_lock_probe")
        .arg("--nocapture")
        .env("EMU_TEST_LOCK_CHILD_PATH", lock_path.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("Failed to spawn child probe process");

    let stdout = child.stdout.take().expect("Failed to open child stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    // Wait for child to observe contention
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .expect("Failed to read child stdout");
        assert!(bytes_read > 0, "Child exited before observing contention");
        if line.contains("CHILD_OBSERVED_CONTENTION") {
            break;
        }
    }

    // Release parent lock
    parent_lock
        .release()
        .expect("Parent failed to release lock");

    // Signal child to acquire
    let mut stdin = child.stdin.take().expect("Failed to open child stdin");
    stdin
        .write_all(b"g")
        .expect("Failed to write to child stdin");
    drop(stdin);

    // Wait for child to acquire
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .expect("Failed to read child stdout");
        assert!(bytes_read > 0, "Child exited before acquiring lock");
        if line.contains("CHILD_ACQUIRED_LOCK") {
            break;
        }
    }

    let status = child.wait().expect("Failed to wait on child");
    assert!(status.success(), "Child probe process failed");

    // Lock file must still exist permanently
    assert!(
        lock_path.exists(),
        "Lockfile must remain permanently on disk"
    );
}

#[tokio::test]
async fn test_advisory_lock_exclusive_and_contention_exit_code() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let lock_path = temp
        .path()
        .join("instances")
        .join("locks")
        .join("vm1.run.lock");

    // First acquisition succeeds
    let lock1 = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("First lock acquisition must succeed");
    assert_eq!(lock1.path(), lock_path);

    // Second acquisition on the same path must fail with Contended (Exit Code 5)
    let err = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect_err("Second acquisition while held must fail with contention");

    assert!(err.is_contended(), "Error must be LockError::Contended");
    assert_eq!(
        err.exit_code(),
        5,
        "Contention exit code must be 5 (EXIT_CONFLICT)"
    );

    // Dropping lock1 releases kernel lock
    drop(lock1);

    // Subsequent acquisition succeeds
    let lock2 = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Acquisition after drop must succeed");
    drop(lock2);
}

#[tokio::test]
async fn test_advisory_lock_explicit_release() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let lock_path = temp
        .path()
        .join("operations")
        .join("locks")
        .join("op1.op.lock");

    let lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Lock acquire failed");

    // Explicit release
    lock.release().expect("Explicit release must succeed");

    // Re-acquire immediately succeeds
    let lock_again = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Re-acquisition after explicit release must succeed");
    drop(lock_again);
}

#[tokio::test]
async fn test_advisory_lock_never_unlinks_or_deletes_files() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let lock_path = temp
        .path()
        .join("instances")
        .join("locks")
        .join("vm_perm.device.lock");

    // Acquire and drop
    let lock = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Lock acquire failed");
    assert!(lock_path.exists(), "Lockfile must exist when held");
    drop(lock);

    // CRITICAL: File must still exist on disk after drop! Never unlinked!
    assert!(
        lock_path.exists(),
        "Lockfile must remain permanently on disk after drop (never unlinked)"
    );

    // Acquire and release explicitly
    let lock2 = AdvisoryLock::try_acquire(&lock_path)
        .await
        .expect("Lock re-acquire failed");
    lock2.release().expect("Release failed");

    // File must still exist on disk after explicit release
    assert!(
        lock_path.exists(),
        "Lockfile must remain permanently on disk after release (never unlinked)"
    );
}

#[tokio::test]
async fn test_advisory_lock_creates_missing_parent_directories_with_0700() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let deeply_nested = temp.path().join("a").join("b").join("c").join("deep.lock");

    let lock = AdvisoryLock::try_acquire(&deeply_nested)
        .await
        .expect("Lock acquisition in non-existent nested path must succeed");
    assert!(deeply_nested.exists());

    #[cfg(unix)]
    {
        let parent = deeply_nested.parent().unwrap();
        let meta = std::fs::metadata(parent).expect("Failed to stat parent");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, SECURE_DIR_MODE,
            "Parent directory mode must be 0700, got 0{mode:o}"
        );
    }

    drop(lock);
}

// ============================================================================
// 3. Runtime Directory (0700) and Socket Path Limits
// ============================================================================

#[tokio::test]
async fn test_runtime_directory_create_default_tmp() {
    let runtime = RuntimeDirectory::create()
        .await
        .expect("RuntimeDirectory::create() failed on default platform temporary directory");

    let path = runtime.path().to_path_buf();
    assert!(path.exists(), "Runtime directory must exist");
    assert!(path.is_dir(), "Runtime path must be a directory");

    let sock_path = runtime
        .socket_path("qmp.sock")
        .expect("Failed to get socket path");
    assert!(
        sock_path.as_os_str().as_encoded_bytes().len() < MAX_DARWIN_SUN_PATH,
        "Socket path must satisfy Darwin 104-byte limit"
    );

    runtime.cleanup().await.expect("cleanup failed");
    assert!(
        !path.exists(),
        "Runtime directory must be removed after cleanup"
    );
}

#[tokio::test]
async fn test_runtime_directory_create_0700_and_cleanup() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let base = temp.path().join("runtime_base");
    std::fs::create_dir(&base).expect("create_dir failed");

    let runtime = RuntimeDirectory::create_under(&base)
        .await
        .expect("RuntimeDirectory creation failed");

    let path = runtime.path().to_path_buf();
    assert!(path.exists(), "Runtime directory must exist");
    assert!(path.is_dir(), "Runtime path must be a directory");

    let meta = std::fs::symlink_metadata(&path).expect("symlink_metadata failed");
    assert!(
        !meta.file_type().is_symlink(),
        "Runtime directory must never be a symlink"
    );

    #[cfg(unix)]
    {
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, SECURE_DIR_MODE,
            "Runtime directory must have mode 0700, got 0{mode:o}"
        );
    }

    // Explicit cleanup
    runtime.cleanup().await.expect("Runtime cleanup failed");
    assert!(
        !path.exists(),
        "Runtime directory must be removed after cleanup"
    );
}

#[tokio::test]
async fn test_runtime_directory_cleanup_with_bound_unix_listener() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let runtime = RuntimeDirectory::create_under(temp.path())
        .await
        .expect("create_under failed");

    let socket_path = runtime
        .socket_path("qmp.sock")
        .expect("Failed to get socket path");

    #[cfg(unix)]
    {
        // Bind real Unix domain socket listener inside runtime directory
        let listener = tokio::net::UnixListener::bind(&socket_path)
            .expect("Failed to bind UnixListener in runtime directory");
        assert!(socket_path.exists(), "Socket file must exist after bind");

        // Cleanup must unlink socket node and remove runtime directory
        runtime
            .cleanup()
            .await
            .expect("Runtime cleanup with bound socket must succeed");
        assert!(!socket_path.exists(), "Socket node must be removed");
        drop(listener);
    }

    #[cfg(not(unix))]
    {
        runtime.cleanup().await.expect("cleanup failed");
    }
}

#[tokio::test]
async fn test_runtime_directory_socket_paths_valid() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let runtime = RuntimeDirectory::create_under(temp.path())
        .await
        .expect("create_under failed");

    let sockets = [
        "qmp.sock",
        "gdb.sock",
        "console.sock",
        "supervisor.sock",
        "inferno-usb.sock",
    ];

    for sock in sockets {
        let sock_path = runtime
            .socket_path(sock)
            .unwrap_or_else(|e| panic!("Failed to get socket path for {sock}: {e}"));
        assert!(sock_path.starts_with(runtime.path()));
        assert_eq!(sock_path.file_name().unwrap(), sock);

        // Path bytes + terminating NUL must be <= 104 bytes
        let total_bytes = sock_path.as_os_str().as_encoded_bytes().len() + 1;
        assert!(
            total_bytes <= MAX_DARWIN_SUN_PATH,
            "Socket path {sock_path:?} total {total_bytes} bytes exceeds Darwin limit {MAX_DARWIN_SUN_PATH}"
        );
    }

    runtime.cleanup().await.expect("cleanup failed");
}

#[tokio::test]
async fn test_runtime_directory_socket_limits_exceeded() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let runtime = RuntimeDirectory::create_under(temp.path())
        .await
        .expect("create_under failed");

    // Create an excessively long socket name that pushes total length + 1 beyond 104 bytes
    let long_name = format!("{}.sock", "x".repeat(95));
    let result = runtime.socket_path(&long_name);

    assert!(result.is_err(), "Overly long socket path must be rejected");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds Darwin limit of 104 bytes"),
        "Error message should mention Darwin 104 byte limit: {err_msg}"
    );

    runtime.cleanup().await.expect("cleanup failed");
}

#[tokio::test]
async fn test_runtime_directory_socket_name_validation() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let runtime = RuntimeDirectory::create_under(temp.path())
        .await
        .expect("create_under failed");

    assert!(
        runtime.socket_path("").is_err(),
        "Empty socket name must fail"
    );
    assert!(
        runtime.socket_path("../evil.sock").is_err(),
        "Directory traversal must fail"
    );
    assert!(
        runtime.socket_path("sub/dir.sock").is_err(),
        "Slash in socket name must fail"
    );
    assert!(
        runtime.socket_path("sub\\dir.sock").is_err(),
        "Backslash in socket name must fail"
    );
    assert!(
        runtime.socket_path("foo\0bar.sock").is_err(),
        "NUL byte in socket name must fail"
    );
    assert!(runtime.socket_path(".").is_err(), "Current dir . must fail");
    assert!(
        runtime.socket_path("..").is_err(),
        "Parent dir .. must fail"
    );

    runtime.cleanup().await.expect("cleanup failed");
}

#[tokio::test]
async fn test_runtime_directory_rejects_arbitrary_symlink_base() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let real_dir = temp.path().join("real");
    std::fs::create_dir(&real_dir).expect("create_dir failed");

    let symlink_dir = temp.path().join("symlink");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_dir, &symlink_dir).expect("symlink failed");

    #[cfg(unix)]
    {
        let result = RuntimeDirectory::create_under(&symlink_dir).await;
        assert!(
            result.is_err(),
            "Runtime directory creation under an arbitrary symlink base must fail"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("untrusted symlink"));
    }
}

// ============================================================================
// 4. ResearchPaths Hierarchy, Containment & Path Traversal Rejection
// ============================================================================

#[tokio::test]
async fn test_security_containment_rejects_symlink_targets_and_preserves_sentinel() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let outside_sentinel_dir = temp.path().join("outside_sentinel_dir");
    std::fs::create_dir(&outside_sentinel_dir).expect("create outside dir failed");
    let sentinel_file = outside_sentinel_dir.join("sentinel.txt");
    std::fs::write(&sentinel_file, b"CONFIDENTIAL_SENTINEL_DATA").expect("write sentinel failed");

    #[cfg(unix)]
    {
        std::fs::set_permissions(&sentinel_file, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(
            &outside_sentinel_dir,
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();

        // 1. Test ResearchPaths::ensure rejects symlinked instance directory
        let research_root = temp.path().join("research");
        std::fs::create_dir_all(&research_root).expect("create research root failed");

        let instances_symlink = research_root.join("instances");
        std::os::unix::fs::symlink(&outside_sentinel_dir, &instances_symlink)
            .expect("symlink failed");

        let paths = ResearchPaths::new(research_root).expect("ResearchPaths::new failed");
        let ensure_result = paths.ensure().await;
        assert!(
            ensure_result.is_err(),
            "ensure() must reject pre-existing symlinked directory"
        );

        // Verify outside sentinel was NOT modified
        assert_eq!(
            std::fs::read(&sentinel_file).unwrap(),
            b"CONFIDENTIAL_SENTINEL_DATA"
        );
        let mode = std::fs::metadata(&outside_sentinel_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o755,
            "Outside sentinel directory permissions must remain untouched"
        );

        // 2. Test AdvisoryLock::try_acquire rejects symlinked lock file
        let lock_symlink = outside_sentinel_dir.join("attack.lock");
        std::os::unix::fs::symlink(&sentinel_file, &lock_symlink).expect("symlink failed");

        let lock_result = AdvisoryLock::try_acquire(&lock_symlink).await;
        assert!(
            lock_result.is_err(),
            "AdvisoryLock::try_acquire must reject symlink lock target"
        );

        // Verify sentinel file content and permissions are still completely untouched
        assert_eq!(
            std::fs::read(&sentinel_file).unwrap(),
            b"CONFIDENTIAL_SENTINEL_DATA"
        );
        let file_mode = std::fs::metadata(&sentinel_file)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            file_mode, 0o644,
            "Sentinel file permissions must remain untouched"
        );
    }
}

#[tokio::test]
async fn test_research_paths_creation_and_ensure() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let root = temp.path().join("research");

    let paths = ResearchPaths::new(root.clone()).expect("ResearchPaths::new failed");
    assert_eq!(paths.root(), root);

    // Call ensure to create directory hierarchy
    paths.ensure().await.expect("paths.ensure failed");

    // Verify all directories exist
    let dirs = [
        paths.root(),
        paths.instances(),
        paths.instance_locks(),
        paths.profiles(),
        paths.artifacts(),
        paths.baselines(),
        paths.records(),
        paths.operations(),
        paths.operation_locks(),
        paths.proposals(),
        paths.security_profiles(),
    ];

    for d in dirs {
        assert!(d.exists(), "Directory '{}' must exist", d.display());
        assert!(d.is_dir(), "Path '{}' must be a directory", d.display());

        #[cfg(unix)]
        {
            let meta = std::fs::metadata(d).expect("stat failed");
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(
                mode,
                SECURE_DIR_MODE,
                "Directory '{}' mode must be 0700, got 0{:o}",
                d.display(),
                mode
            );
        }
    }

    // Verify nested lock paths adhere to canonical data model
    assert_eq!(paths.instance_locks(), paths.instances().join("locks"));
    assert_eq!(paths.operation_locks(), paths.operations().join("locks"));
}

#[test]
fn test_research_paths_rejects_relative_root() {
    let relative = PathBuf::from("relative/emu/research");
    let result = ResearchPaths::new(relative);
    assert!(result.is_err(), "Relative root path must be rejected");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must be an absolute path")
    );
}

#[test]
fn test_validate_path_component_traversal_rejection() {
    // Valid components
    assert_eq!(
        validate_path_component("valid-id-123").unwrap(),
        "valid-id-123"
    );
    assert_eq!(
        validate_path_component("art_sha256_abcd").unwrap(),
        "art_sha256_abcd"
    );
    assert_eq!(
        validate_path_component("rec_550e8400").unwrap(),
        "rec_550e8400"
    );

    // Traversal attempts
    assert_eq!(
        validate_path_component("../evil"),
        Err(PathError::Traversal("../evil".to_string()))
    );
    assert_eq!(
        validate_path_component(".."),
        Err(PathError::Traversal("..".to_string()))
    );
    assert_eq!(
        validate_path_component("."),
        Err(PathError::Traversal(".".to_string()))
    );
    assert_eq!(
        validate_path_component("/absolute/path"),
        Err(PathError::Traversal("/absolute/path".to_string()))
    );
    assert_eq!(
        validate_path_component("foo/bar"),
        Err(PathError::Traversal("foo/bar".to_string()))
    );
    assert_eq!(
        validate_path_component("foo\\bar"),
        Err(PathError::Traversal("foo\\bar".to_string()))
    );

    // Invalid characters
    assert_eq!(
        validate_path_component("foo\0bar"),
        Err(PathError::InvalidComponent("foo\0bar".to_string()))
    );
    assert_eq!(validate_path_component(""), Err(PathError::EmptyComponent));
}

#[test]
fn test_research_paths_resolvers_and_traversal_guards() {
    let temp = tempfile::tempdir().expect("Failed to create tempdir");
    let paths = ResearchPaths::new(temp.path().to_path_buf()).unwrap();

    // Valid resolutions
    assert_eq!(
        paths.instance_path("guest1").unwrap(),
        paths.instances().join("guest1.json")
    );
    assert_eq!(
        paths.instance_run_lock_path("guest1").unwrap(),
        paths.instance_locks().join("guest1.run.lock")
    );
    assert_eq!(
        paths.instance_device_lock_path("guest1").unwrap(),
        paths.instance_locks().join("guest1.device.lock")
    );
    assert_eq!(
        paths.profile_path("prof1").unwrap(),
        paths.profiles().join("prof1.json")
    );
    assert_eq!(
        paths.artifact_path("sha256_art").unwrap(),
        paths.artifacts().join("sha256_art.json")
    );
    assert_eq!(
        paths.baseline_path("base1").unwrap(),
        paths.baselines().join("base1.json")
    );
    assert_eq!(
        paths.record_path("rec1").unwrap(),
        paths.records().join("rec1.json")
    );
    assert_eq!(
        paths.operation_path("op_1").unwrap(),
        paths.operations().join("op_1.json")
    );
    assert_eq!(
        paths.operation_events_path("op_1").unwrap(),
        paths.operations().join("op_1.events.jsonl")
    );
    assert_eq!(
        paths.operation_lock_path("op_1").unwrap(),
        paths.operation_locks().join("op_1.op.lock")
    );
    assert_eq!(
        paths.proposal_path("sha256_prop").unwrap(),
        paths.proposals().join("sha256_prop.json")
    );
    assert_eq!(
        paths.security_profile_path("sec1").unwrap(),
        paths.security_profiles().join("sec1.json")
    );

    // Traversal rejections on resolvers
    assert!(matches!(
        paths.instance_path("../bad"),
        Err(PathError::Traversal(_))
    ));
    assert!(matches!(
        paths.instance_run_lock_path("../../etc/passwd"),
        Err(PathError::Traversal(_))
    ));
    assert!(matches!(
        paths.operation_lock_path("/root"),
        Err(PathError::Traversal(_))
    ));
    assert!(matches!(
        paths.proposal_path("foo\0bar"),
        Err(PathError::InvalidComponent(_))
    ));
}
