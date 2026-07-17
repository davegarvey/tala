use std::path::PathBuf;
use std::process::Command;

fn chit_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_chit"))
}

fn chit(home: &std::path::Path, args: &[&str]) -> (String, String, bool) {
    chit_in(home, None, args)
}

fn chit_in(
    home: &std::path::Path,
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> (String, String, bool) {
    let mut cmd = Command::new(chit_bin());
    cmd.env("HOME", home).args(args);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run chit {}: {}", args.join(" "), e));

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn chit_ok(home: &std::path::Path, args: &[&str]) -> String {
    let (stdout, stderr, ok) = chit(home, args);
    assert!(
        ok,
        "chit {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        stdout,
        stderr
    );
    stdout
}

fn chit_start(home: &std::path::Path) -> String {
    let stdout = chit_ok(home, &["start"]);
    stdout.trim().to_string()
}

fn chit_stop(home: &std::path::Path) {
    let _ = chit(home, &["stop"]);
}

#[test]
fn test_daemon_lifecycle() {
    let home = tempfile::tempdir().unwrap();

    let session = chit_start(home.path());
    assert!(
        session.starts_with("sess_"),
        "session should start with sess_"
    );

    let status = chit_ok(home.path(), &["status"]);
    assert!(
        status.contains("daemon running"),
        "status should show daemon: {}",
        status
    );
    assert!(status.contains("PID:"), "status should show PID");

    let list = chit_ok(home.path(), &["list"]);
    assert!(list.contains(&session), "list should show session");

    chit_stop(home.path());

    let status = chit_ok(home.path(), &["status"]);
    assert!(
        status.contains("no daemon"),
        "status should show no daemon after stop"
    );
}

#[test]
fn test_send_and_recap() {
    let home = tempfile::tempdir().unwrap();

    let session = chit_start(home.path());

    chit_ok(
        home.path(),
        &["send", "--session", &session, "--ff", "Hello from **test**"],
    );

    let recap = chit_ok(home.path(), &["recap", &session]);
    assert!(
        recap.contains("Hello from **test**"),
        "recap should contain message"
    );
    assert!(recap.contains(&session), "recap should show session ID");

    chit_stop(home.path());
}

#[test]
fn test_auto_target_single_session() {
    let home = tempfile::tempdir().unwrap();

    chit_start(home.path());

    chit_ok(home.path(), &["send", "--ff", "auto-target test"]);

    let recap = chit_ok(home.path(), &["recap"]);
    assert!(
        recap.contains("auto-target test"),
        "recap should contain message via auto-target"
    );

    chit_stop(home.path());
}

#[test]
fn test_multiple_sessions_auto_target_error() {
    let home = tempfile::tempdir().unwrap();

    chit_start(home.path());
    chit_start(home.path());

    let (_stdout, stderr, ok) = chit(home.path(), &["send", "--ff", "test"]);
    assert!(!ok, "send should fail with multiple sessions");
    assert!(
        stderr.contains("Multiple active sessions"),
        "error should list multiple sessions: {}",
        stderr
    );

    chit_stop(home.path());
}

fn run_init_in(dir: &std::path::Path, home: &std::path::Path, args: &[&str]) {
    let (stdout, stderr, ok) = chit_in(home, Some(dir), args);
    assert!(
        ok,
        "chit {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        stdout,
        stderr
    );
}

#[test]
fn test_init_command() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    run_init_in(project.path(), home.path(), &["init"]);

    let config_path = project.path().join(".chit").join("config.json");
    assert!(
        config_path.exists(),
        "init should create .chit/config.json: {:?}",
        config_path
    );

    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("name"), "config should contain name field");
}

#[test]
fn test_init_with_custom_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    run_init_in(
        project.path(),
        home.path(),
        &["init", "--name", "my-custom-project"],
    );

    let config_path = project.path().join(".chit").join("config.json");
    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config.contains("my-custom-project"),
        "config should contain custom name"
    );
}

#[test]
fn test_init_detects_opencode_harness() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".opencode")).unwrap();

    run_init_in(project.path(), home.path(), &["init"]);

    let skill_path = project
        .path()
        .join(".opencode")
        .join("skills")
        .join("chit")
        .join("SKILL.md");
    assert!(
        skill_path.exists(),
        "init should detect .opencode/ and create skill file at .opencode/skills/chit/SKILL.md"
    );

    let skill = std::fs::read_to_string(&skill_path).unwrap();
    assert!(
        skill.contains("name: chit"),
        "skill should have YAML frontmatter with name"
    );
    assert!(skill.contains("chit"), "skill should reference chit");

    let command_path = project
        .path()
        .join(".opencode")
        .join("commands")
        .join("chit.md");
    assert!(
        command_path.exists(),
        "init should detect .opencode/ and create command file at .opencode/commands/chit.md"
    );
}

#[test]
fn test_init_does_not_install_opencode_skills_without_harness() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    run_init_in(project.path(), home.path(), &["init"]);

    let skill_path = project
        .path()
        .join(".opencode")
        .join("skills")
        .join("chit")
        .join("SKILL.md");
    assert!(
        !skill_path.exists(),
        "init should not install opencode skills without .opencode/ dir"
    );
}

#[test]
fn test_close_session() {
    let home = tempfile::tempdir().unwrap();

    let session = chit_start(home.path());

    let close = chit_ok(home.path(), &["close", &session]);
    assert!(close.contains("closed"), "close should confirm: {}", close);

    let list = chit_ok(home.path(), &["list"]);
    assert!(
        list.contains("closed"),
        "list should show session as closed"
    );

    chit_stop(home.path());
}

#[test]
fn test_agent_to_agent_conversation() {
    let home = tempfile::tempdir().unwrap();

    let session = chit_start(home.path());

    chit_ok(
        home.path(),
        &[
            "send",
            "--session",
            &session,
            "--ff",
            "Bug in grubble: fix scope commits",
        ],
    );

    chit_ok(
        home.path(),
        &[
            "send",
            "--session",
            &session,
            "--ff",
            "--as",
            "grubble-agent",
            "Found it, fix pushed",
        ],
    );

    let recap = chit_ok(home.path(), &["recap", &session]);
    assert!(
        recap.contains("Bug in grubble"),
        "recap should have first message"
    );
    assert!(
        recap.contains("Found it"),
        "recap should have second message"
    );
    assert!(
        recap.contains("grubble-agent"),
        "recap should attribute --as name"
    );

    chit_stop(home.path());
}

#[test]
fn test_chit_start_with_message() {
    let home = tempfile::tempdir().unwrap();

    let sess = chit_start(home.path());

    chit_ok(
        home.path(),
        &["send", "--session", &sess, "--ff", "Starting message test"],
    );

    let recap = chit_ok(home.path(), &["recap", &sess]);
    assert!(recap.contains("Starting message test"));

    chit_stop(home.path());
}

#[test]
fn test_wait_timeout() {
    let home = tempfile::tempdir().unwrap();

    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &["wait", &sess, "--timeout", "2"]);
    assert!(
        ok || (!ok && stdout.contains("timeout")),
        "wait timeout should succeed or report timeout with code 2: {}",
        stdout
    );
    assert!(
        stdout.contains("timeout"),
        "wait should report timeout: {}",
        stdout
    );

    chit_stop(home.path());
}

#[test]
fn test_wait_since_returns_existing_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "existing-msg"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "wait", &sess, "--since", "0", "--timeout", "3", "--json",
    ]);
    assert!(ok, "wait --since should succeed");
    assert!(stdout.contains("existing-msg"), "should return existing msg");
    assert!(stdout.contains("\"cursor\""), "should include cursor");

    chit_stop(home.path());
}

#[test]
fn test_wait_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "alpha", "msg-alpha"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "beta", "msg-beta"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "wait", &sess, "--since", "0", "--from", "alpha", "--timeout", "3", "--json",
    ]);
    assert!(ok, "wait --from should succeed");
    assert!(stdout.contains("alpha"), "should include alpha");
    assert!(!stdout.contains("beta"), "should exclude beta");

    chit_stop(home.path());
}

#[test]
fn test_wait_limit_cap() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "t", "m1"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "t", "m2"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "t", "m3"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "wait", &sess, "--since", "0", "--from", "t", "--limit", "2", "--timeout", "3", "--json",
    ]);
    assert!(ok, "wait --limit should succeed");

    let count = stdout.matches("\"content\"").count();
    assert_eq!(count, 2, "should cap at 2 messages: {}", stdout);

    chit_stop(home.path());
}

#[test]
fn test_recap_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "alpha", "only-alpha"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "beta", "only-beta"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "recap", &sess, "--json", "--from", "alpha",
    ]);
    assert!(ok, "recap --from should succeed");
    assert!(stdout.contains("only-alpha"), "should include alpha msg");
    assert!(!stdout.contains("only-beta"), "should exclude beta msg");

    chit_stop(home.path());
}

#[test]
fn test_recap_cursor() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "old-msg"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "new-msg"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "recap", &sess, "--json", "--cursor", "1",
    ]);
    assert!(ok, "recap --cursor should succeed");
    assert!(!stdout.contains("old-msg"), "should exclude old-msg");
    assert!(stdout.contains("new-msg"), "should include new-msg");

    chit_stop(home.path());
}

#[test]
fn test_recap_limit_cap() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "m1"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "m2"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "m3"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "recap", &sess, "--json", "--limit", "2",
    ]);
    assert!(ok, "recap --limit should succeed");
    let count = stdout.matches("\"content\"").count();
    assert_eq!(count, 2, "should cap at 2 messages");

    chit_stop(home.path());
}

#[test]
fn test_recap_limit_zero_is_unlimited() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "a"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "b"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "c"]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "recap", &sess, "--json", "--limit", "0",
    ]);
    assert!(ok, "recap --limit 0 should succeed");
    let count = stdout.matches("\"content\"").count();
    assert!(count >= 3, "limit 0 should return all messages, got {}", count);

    chit_stop(home.path());
}

#[test]
fn test_send_json_output() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--ff", "--json", "json-test",
    ]);
    assert!(ok, "send --json should succeed");
    assert!(stdout.contains("\"cursor\""), "should include cursor");
    assert!(stdout.contains("\"content\""), "should include content");

    chit_stop(home.path());
}

#[test]
fn test_close_json_output() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "close", &sess, "--json",
    ]);
    assert!(ok, "close --json should succeed");
    assert!(stdout.contains("\"status\""), "should include status");
    assert!(stdout.contains("closed"), "status should be closed");

    chit_stop(home.path());
}

#[test]
fn test_status_json_output() {
    let home = tempfile::tempdir().unwrap();
    chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &["status", "--json"]);
    assert!(ok, "status --json should succeed");
    assert!(stdout.contains("\"pid\""), "should include pid");

    chit_stop(home.path());
}

#[test]
fn test_list_json_output() {
    let home = tempfile::tempdir().unwrap();
    chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &["list", "--json"]);
    assert!(ok, "list --json should succeed");
    assert!(stdout.contains("\"session_id\""), "should include session_id");

    chit_stop(home.path());
}

#[test]
fn test_send_to_closed_session_fails() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["close", &sess]);

    let (_stdout, stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--ff", "this should fail",
    ]);
    assert!(!ok, "send to closed should fail");
    assert!(stderr.contains("closed"), "error should mention closed");

    chit_stop(home.path());
}

#[test]
fn test_close_already_closed_fails() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["close", &sess]);
    let (_stdout, _stderr, ok) = chit(home.path(), &["close", &sess]);
    assert!(!ok, "close already-closed should fail");

    chit_stop(home.path());
}

#[test]
fn test_wait_after_close_returns_messages_and_closed_true() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "pending-msg"]);
    chit_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "wait", &sess, "--since", "0", "--timeout", "3", "--json",
    ]);
    assert!(ok, "wait after close should succeed");
    assert!(stdout.contains("\"closed\":true"), "should report closed:true");
    assert!(stdout.contains("pending-msg"), "should return pending messages");

    chit_stop(home.path());
}

#[test]
fn test_follow_after_close() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    chit_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "follow", "--session", &sess, "--since", "0", "--timeout", "3", "--json",
    ]);
    assert!(ok, "follow after close should succeed");
    assert!(stdout.contains("closed"), "should emit closed event");

    chit_stop(home.path());
}

#[test]
fn test_empty_message_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (_stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--ff", "",
    ]);
    assert!(!ok, "empty message should be rejected");

    chit_stop(home.path());
}

#[test]
fn test_empty_session_name_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (_stdout, _stderr, ok) = chit(home.path(), &[
        "session", "rename", &sess, "",
    ]);
    assert!(!ok, "empty session name should be rejected");

    chit_stop(home.path());
}

#[test]
fn test_session_rename_and_show() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let close = chit_ok(home.path(), &["session", "rename", &sess, "my-project"]);
    assert!(close.contains("renamed"), "rename should confirm");

    let show = chit_ok(home.path(), &["session", "show", &sess]);
    assert!(show.contains("my-project"), "show should display name");

    chit_stop(home.path());
}

#[test]
fn test_session_close_alias() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let out = chit_ok(home.path(), &["session", "close", &sess]);
    assert!(out.contains("closed"), "session close should confirm");

    chit_stop(home.path());
}

#[test]
fn test_nonexistent_session_recap_fails() {
    let home = tempfile::tempdir().unwrap();
    chit_start(home.path());

    let (_stdout, _stderr, ok) = chit(home.path(), &["recap", "nonexistent"]);
    assert!(!ok, "recap nonexistent should fail");

    chit_stop(home.path());
}

#[test]
fn test_nonexistent_session_wait_fails() {
    let home = tempfile::tempdir().unwrap();
    chit_start(home.path());

    let (_stdout, _stderr, ok) = chit(home.path(), &[
        "wait", "nonexistent", "--timeout", "2",
    ]);
    assert!(!ok, "wait nonexistent should fail");

    chit_stop(home.path());
}

#[test]
fn test_no_wait_flag_instead_of_ff() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--no-wait", "sent with --no-wait",
    ]);
    assert!(ok, "--no-wait should work");
    assert!(stdout.contains("Sent message"), "should show confirmation: {}", stdout);

    let recap = chit_ok(home.path(), &["recap", &sess]);
    assert!(recap.contains("--no-wait"), "message should be in recap");

    chit_stop(home.path());
}

#[test]
fn test_ff_alias_still_works() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--ff", "sent with old --ff alias",
    ]);
    assert!(ok, "--ff alias should still work");
    assert!(stdout.contains("Sent message"), "should show confirmation: {}", stdout);

    let recap = chit_ok(home.path(), &["recap", &sess]);
    assert!(recap.contains("old --ff"), "message should be in recap");

    chit_stop(home.path());
}

#[test]
fn test_send_quiet_flag() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--no-wait", "--quiet", "quiet message",
    ]);
    assert!(ok, "--quiet should still succeed");
    assert!(!stdout.contains("Sent"), "should not print confirmation: {:?}", stdout);

    chit_stop(home.path());
}

#[test]
fn test_send_file_flag() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let msg_path = home.path().join("msg.txt");
    std::fs::write(&msg_path, "message from file with **markdown**").unwrap();

    let (stdout, _stderr, ok) = chit(home.path(), &[
        "send", "--session", &sess, "--no-wait", "--file",
        msg_path.to_str().unwrap(),
    ]);
    assert!(ok, "--file should work");
    assert!(stdout.contains("Sent message"), "should show confirmation");

    let recap = chit_ok(home.path(), &["recap", &sess, "--json"]);
    assert!(recap.contains("**markdown**"), "file content should be in recap");

    chit_stop(home.path());
}

#[test]
fn test_use_set_and_clear() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    // Set active session
    let out = chit_ok(home.path(), &["use", &sess]);
    assert!(out.contains("Active session set"), "should confirm: {}", out);

    // Show active session
    let out = chit_ok(home.path(), &["use"]);
    assert!(out.contains(&sess), "should show session: {}", out);

    // Send without --session should use active session
    chit_ok(home.path(), &["send", "--no-wait", "sent via active session"]);

    let recap = chit_ok(home.path(), &["recap", &sess]);
    assert!(recap.contains("active session"), "message should be in session");

    // Clear
    let out = chit_ok(home.path(), &["use", "--clear"]);
    assert!(out.contains("cleared"), "should confirm clear: {}", out);

    // Verify cleared
    let (stdout, _stderr, ok) = chit(home.path(), &["use"]);
    assert!(ok);
    assert!(!stdout.contains(&sess), "should not show session after clear");

    chit_stop(home.path());
}

#[test]
fn test_use_json_output() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let out = chit_ok(home.path(), &["use", &sess, "--json"]);
    assert!(out.contains("\"session_id\""), "json should have session_id: {}", out);
    assert!(out.contains(&sess), "json should contain session id");

    let out = chit_ok(home.path(), &["use", "--json"]);
    assert!(out.contains("\"session_id\""), "json show should have session_id");

    let out = chit_ok(home.path(), &["use", "--clear", "--json"]);
    assert!(out.contains("\"status\""), "json clear should have status");

    chit_stop(home.path());
}

#[test]
fn test_send_stdin() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(chit_bin())
        .env("HOME", home.path())
        .args(&["send", "--session", &sess, "--no-wait", "--quiet", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start chit");

    child.stdin.take().unwrap().write_all(b"piped stdin message").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "stdin send should succeed");

    let recap = chit_ok(home.path(), &["recap", &sess]);
    assert!(recap.contains("piped stdin message"), "stdin message should be in recap");

    chit_stop(home.path());
}

#[test]
fn test_follow_streams_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let mut child = Command::new(chit_bin())
        .env("HOME", home.path())
        .args(&["follow", "--session", &sess, "--since", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start follow");

    std::thread::sleep(std::time::Duration::from_millis(500));

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "--as", "streamer", "live-msg"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("live-msg"), "follow should stream msg: {}", stdout);
    assert!(stdout.contains("streamer"), "follow should show sender");

    chit_stop(home.path());
}

#[test]
fn test_follow_limit_caps_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let mut child = Command::new(chit_bin())
        .env("HOME", home.path())
        .args(&["follow", "--session", &sess, "--since", "0", "--limit", "1", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start follow");

    std::thread::sleep(std::time::Duration::from_millis(500));

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "limit-1-a"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "limit-1-b"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.matches("\"content\"").count();
    assert_eq!(count, 1, "follow --limit 1 should cap at 1: {}", stdout);

    chit_stop(home.path());
}

#[test]
fn test_follow_limit_zero_is_unlimited() {
    let home = tempfile::tempdir().unwrap();
    let sess = chit_start(home.path());

    let mut child = Command::new(chit_bin())
        .env("HOME", home.path())
        .args(&["follow", "--session", &sess, "--since", "0", "--limit", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start follow");

    std::thread::sleep(std::time::Duration::from_millis(500));

    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "unlim-a"]);
    chit_ok(home.path(), &["send", "--session", &sess, "--ff", "unlim-b"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unlim-a"), "limit 0 should show unlim-a");
    assert!(stdout.contains("unlim-b"), "limit 0 should show unlim-b");

    chit_stop(home.path());
}
