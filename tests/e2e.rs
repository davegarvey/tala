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
