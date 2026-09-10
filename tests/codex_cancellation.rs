use std::process::Command;

#[test]
fn real_idle_child_is_reaped_on_cancellation() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("agentmeter-cancel-{}-{unique}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cancellation_child", "--nocapture"])
        .env("AGENTMETER_CANCELLATION_PROBE", "1")
        .env("AGENTMETER_FAKE_MODE", "cancellable_idle")
        .env("AGENTMETER_FAKE_READY", dir.join("ready"))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn cancellation_child() {
    if std::env::var_os("AGENTMETER_CANCELLATION_PROBE").is_none() {
        return;
    }
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    let worker = std::thread::spawn(move || {
        agentmeter_p0::codex::collect_live_cancellable(
            std::path::Path::new(env!("CARGO_BIN_EXE_fake-codex-app-server")),
            Duration::from_secs(30),
            &signal,
        )
    });
    let marker = std::path::PathBuf::from(std::env::var_os("AGENTMETER_FAKE_READY").unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let ready = marker.exists();
    let start = Instant::now();
    cancelled.store(true, Ordering::Release);
    let failure = worker.join().unwrap().unwrap_err();
    assert!(ready, "fixture did not reach running state");
    assert_eq!(failure.failure_code, "cancelled");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "cancellation waited for the 30-second deadline"
    );
    // Windows fixture holds a no-sharing handle for its whole lifetime.
    // Successful reopen proves it is no longer holding the resource after return.
    let released =
        std::fs::File::open(marker).expect("child resource still locked after cancellation");
    drop(released);
}
