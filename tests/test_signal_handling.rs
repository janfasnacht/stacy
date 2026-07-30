/// Tests for signal handling (SIGINT, SIGTERM, SIGKILL)
///
/// Verifies that stacy correctly handles process interruption
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
#[ignore] // Manual test - requires Ctrl-C
fn test_ctrl_c_manual() {
    // This test documents the expected behavior when user presses Ctrl-C
    //
    // To test manually:
    // 1. cargo run -- run tests/log-analysis/07_infinite_loop.do -v
    // 2. Press Ctrl-C after a few seconds
    //
    // Expected:
    // - Process terminates immediately
    // - Exit code: 130 (128 + SIGINT signal number 2)
    // - Log shows partial output
    //
    // Implementation in src/executor/runner.rs:
    // - Spawns child process
    // - On Ctrl-C, child receives SIGINT
    // - Exit status extracts signal: 128 + signal_number
}

#[test]
#[ignore] // Requires manual process management
fn test_sigterm_handling() {
    // Test that SIGTERM is properly handled
    //
    // This happens when:
    // - Timeout kills the process
    // - System sends TERM signal
    // - Another process kills stacy
    //
    // Expected exit code: 143 (128 + 15)
    //
    // Tested in test_timeout.rs::test_infinite_loop_with_timeout
}

#[test]
fn test_signal_exit_codes() {
    // Document the signal to exit code mapping
    //
    // Implementation in src/executor/runner.rs:exit_code_from_status
    //
    // SIGINT (Ctrl-C):   128 + 2  = 130
    // SIGTERM (timeout): 128 + 15 = 143
    // SIGKILL (force):   128 + 9  = 137
    //
    // These are standard Unix conventions

    let sigint_code = 128 + 2;
    let sigterm_code = 128 + 15;
    let sigkill_code = 128 + 9;

    assert_eq!(sigint_code, 130, "SIGINT exit code");
    assert_eq!(sigterm_code, 143, "SIGTERM exit code");
    assert_eq!(sigkill_code, 137, "SIGKILL exit code");
}

/// A signal to stacy must reach the run it started. Stata is in its own process
/// group now (#118), so the terminal no longer delivers Ctrl-C to it and stacy
/// forwards the signal instead. Without that, killing stacy would leave Stata
/// running — which is the bug this guards.
///
/// SIGTERM stands in for Ctrl-C here: a shell's background jobs ignore SIGINT
/// when job control is off, which would make the stub, not stacy, decide the
/// outcome. The forwarding path is the same for both signals; the exit status
/// Ctrl-C produces is covered by the test below.
#[cfg(unix)]
#[test]
fn test_signal_reaches_processes_the_run_spawned() {
    let temp = tempfile::TempDir::new().unwrap();
    let marker = temp.path().join("survivor.marker");
    let stub = write_stub(temp.path(), &marker);
    std::fs::write(temp.path().join("dummy.do"), "display 1\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_stacy"))
        .args(["run", "dummy.do", "--engine"])
        .arg(&stub)
        .current_dir(temp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn stacy");

    // Let the run get as far as spawning its descendant.
    thread::sleep(Duration::from_millis(800));

    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let _ = child.wait().expect("Failed to wait");

    // Outlast the descendant's own schedule, so a survivor has every chance to
    // write the marker.
    thread::sleep(Duration::from_secs(4));

    assert!(
        !marker.exists(),
        "a process the run spawned outlived stacy — the signal was not forwarded"
    );
}

/// Forwarding must not change the status stacy itself exits with: Ctrl-C still
/// has to look like Ctrl-C to whatever started stacy.
#[cfg(unix)]
#[test]
fn test_sigint_still_kills_stacy_with_sigint() {
    use std::os::unix::process::ExitStatusExt;

    let temp = tempfile::TempDir::new().unwrap();
    let marker = temp.path().join("unused.marker");
    let stub = write_stub(temp.path(), &marker);
    std::fs::write(temp.path().join("dummy.do"), "display 1\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_stacy"))
        .args(["run", "dummy.do", "--engine"])
        .arg(&stub)
        .current_dir(temp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn stacy");

    thread::sleep(Duration::from_millis(800));

    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let status = child.wait().expect("Failed to wait");

    assert_eq!(
        status.signal(),
        Some(libc::SIGINT),
        "stacy should still die of the signal it was sent, giving callers 130"
    );
}

/// A stand-in for Stata that leaves a process behind, like `stata-mp` does. The
/// descendant announces itself by writing `marker` three seconds in, so a
/// survivor is visible without depending on pid lifetimes.
#[cfg(unix)]
fn write_stub(dir: &std::path::Path, marker: &std::path::Path) -> std::path::PathBuf {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let stub = dir.join("stata_stub.sh");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(&stub)
        .unwrap();
    write!(
        f,
        "#!/bin/sh\n( sleep 3; echo alive > {} ) &\nwait\n",
        marker.display()
    )
    .unwrap();
    f.sync_all().unwrap();
    stub
}

#[test]
#[ignore] // Slow test - spawns external process
fn test_background_kill() {
    // Test killing a running stacy process from another terminal
    //
    // Steps:
    // 1. Start: stacy run tests/log-analysis/07_infinite_loop.do
    // 2. Get PID: ps aux | grep stacy
    // 3. Kill: kill -TERM <pid>
    // 4. Verify exit code 143
    //
    // Or:
    // 1. Start stacy in background
    // 2. Send SIGTERM programmatically
    // 3. Check exit code

    let mut child = Command::new("./target/debug/stacy")
        .args(["run", "tests/log-analysis/07_infinite_loop.do", "--quiet"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn stacy");

    // Let it start
    thread::sleep(Duration::from_millis(500));

    // Kill with SIGTERM
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }
    }

    // Wait for termination
    let status = child.wait().expect("Failed to wait");

    // Should exit with SIGTERM code
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(15), "Should be killed by SIGTERM");
    }
}
