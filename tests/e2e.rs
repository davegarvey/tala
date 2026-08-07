use std::path::PathBuf;
use std::process::Command;

fn tala_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tala"))
}

fn tala(home: &std::path::Path, args: &[&str]) -> (String, String, bool) {
    tala_in(home, None, args)
}

fn tala_in(
    home: &std::path::Path,
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> (String, String, bool) {
    let mut cmd = Command::new(tala_bin());
    cmd.env("HOME", home).args(args);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run tala {}: {}", args.join(" "), e));

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn tala_ok(home: &std::path::Path, args: &[&str]) -> String {
    let (stdout, stderr, ok) = tala(home, args);
    assert!(
        ok,
        "tala {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        stdout,
        stderr
    );
    stdout
}

fn tala_start(home: &std::path::Path) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let project = home.join(format!("p-{}", n));
    std::fs::create_dir_all(&project).unwrap();
    let (stdout, _stderr, ok) = tala_in(home, Some(&project), &["session", "create"]);
    assert!(
        ok,
        "tala session create failed\nstdout: {}\nstderr: {}",
        stdout, _stderr
    );
    stdout.lines().next().unwrap_or("").trim().to_string()
}

fn tala_stop(home: &std::path::Path) {
    let _ = tala(home, &["stop"]);
}

#[test]
fn test_daemon_lifecycle() {
    let home = tempfile::tempdir().unwrap();

    let session = tala_start(home.path());
    assert!(
        session.starts_with("sess_"),
        "session should start with sess_"
    );

    let status = tala_ok(home.path(), &["status"]);
    assert!(
        status.contains("daemon running"),
        "status should show daemon: {}",
        status
    );
    assert!(status.contains("PID:"), "status should show PID");

    let list = tala_ok(home.path(), &["list"]);
    assert!(list.contains(&session), "list should show session");

    tala_stop(home.path());

    let status = tala_ok(home.path(), &["status"]);
    assert!(
        status.contains("no daemon"),
        "status should show no daemon after stop"
    );
}

#[test]
fn test_send_and_recap() {
    let home = tempfile::tempdir().unwrap();

    let session = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &session, "Hello from **test**"],
    );

    let recap = tala_ok(home.path(), &["history", &session]);
    assert!(
        recap.contains("Hello from **test**"),
        "recap should contain message"
    );
    assert!(recap.contains(&session), "recap should show session ID");

    tala_stop(home.path());
}

#[test]
fn test_send_rejects_unknown_flags() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    // Make sess active from the project dir (isolated from parallel tests)
    tala_in(home.path(), Some(project.path()), &["use", &sess]);

    // Unknown flag must be a hard error, not silently sent to the active session
    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--new", "does --new create a session?"],
    );
    assert!(!ok, "unknown flag --new must fail; got stdout: {}", stdout);
    assert!(
        stderr.contains("unexpected argument") || stdout.contains("unexpected argument"),
        "error should mention unexpected argument: stderr={}",
        stderr
    );

    // Nothing may have been sent to the active session
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap.contains("does --new create a session?"),
        "message must NOT be sent when an unknown flag is present: {}",
        recap
    );

    // Typo'd flag with a value: neither the flag nor its value may be sent
    let (stdout2, stderr2, ok2) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--timeot", "1"],
    );
    assert!(
        !ok2,
        "typo flag --timeot must fail; got stdout: {}",
        stdout2
    );
    let recap2 = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap2.contains("\n    1\n") && !recap2.contains("\n    1"),
        "typo flag value must NOT be sent as message content: {}",
        recap2
    );
    assert!(
        stderr2.contains("unexpected argument") || stdout2.contains("unexpected argument"),
        "error should mention unexpected argument: stderr={}",
        stderr2
    );

    // Non-sess_ positional alongside a message must error, not silently drop the target
    let (stdout3, _stderr3, ok3) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "not-a-session", "msg"],
    );
    assert!(
        !ok3,
        "non-session target must fail; got stdout: {}",
        stdout3
    );
    let recap3 = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap3.contains("\n    msg"),
        "message must NOT be sent when the target is not a session: {}",
        recap3
    );

    tala_stop(home.path());
}

#[test]
fn test_send_dash_separator_and_explicit_session_still_work() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    tala_in(home.path(), Some(project.path()), &["use", &sess]);

    // Documented `--` separator still delivers dashed content
    tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--", "--dashed-content"],
    );
    let recap = tala_in(home.path(), Some(project.path()), &["history", &sess]).0;
    assert!(
        recap.contains("--dashed-content"),
        "-- separator must still deliver dashed content: {}",
        recap
    );

    // Explicit -s/--session flag still works
    tala_in(
        home.path(),
        Some(project.path()),
        &["send", "-s", &sess, "explicit flag send"],
    );
    let recap2 = tala_in(home.path(), Some(project.path()), &["history", &sess]).0;
    assert!(
        recap2.contains("explicit flag send"),
        "-s flag must still work: {}",
        recap2
    );

    tala_stop(home.path());
}

#[test]
fn test_auto_target_single_session() {
    let home = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "auto-target test"],
    );

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("auto-target test"),
        "recap should contain message via auto-target: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_multiple_sessions_auto_target_sends_to_active() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess1 = tala_start(home.path());
    let sess2 = tala_start(home.path());

    // Use a project-specific dir so .tala/active-session is isolated per test
    // (parallel tests all share the same CWD, which would cause races)
    tala_in(home.path(), Some(project.path()), &["use", &sess2]);
    tala_in(home.path(), Some(project.path()), &["send", "test"]);
    let recap = tala_ok(home.path(), &["history", &sess2]);
    assert!(
        recap.contains("test"),
        "message should go to active session (sess2)"
    );

    // Explicit --session still works for other sessions
    tala_ok(home.path(), &["send", "--session", &sess1, "explicit send"]);
    let recap2 = tala_ok(home.path(), &["history", &sess1]);
    assert!(
        recap2.contains("explicit send"),
        "explicit send to sess1 should work"
    );

    tala_stop(home.path());
}

fn run_init_in(dir: &std::path::Path, home: &std::path::Path, args: &[&str]) {
    let (stdout, stderr, ok) = tala_in(home, Some(dir), args);
    assert!(
        ok,
        "tala {} failed\nstdout: {}\nstderr: {}",
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

    let config_path = project.path().join(".tala").join("config.json");
    assert!(
        config_path.exists(),
        "init should create .tala/config.json: {:?}",
        config_path
    );

    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("name"), "config should contain name field");
}

#[test]
fn test_init_with_custom_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    run_init_in(project.path(), home.path(), &["init", "my-custom-project"]);

    let config_path = project.path().join(".tala").join("config.json");
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
        .join("tala")
        .join("SKILL.md");
    assert!(
        skill_path.exists(),
        "init should detect .opencode/ and create skill file at .opencode/skills/tala/SKILL.md"
    );

    let skill = std::fs::read_to_string(&skill_path).unwrap();
    assert!(
        skill.contains("name: tala"),
        "skill should have YAML frontmatter with name"
    );
    assert!(skill.contains("tala"), "skill should reference tala");

    let command_path = project
        .path()
        .join(".opencode")
        .join("commands")
        .join("tala.md");
    assert!(
        command_path.exists(),
        "init should detect .opencode/ and create command file at .opencode/commands/tala.md"
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
        .join("tala")
        .join("SKILL.md");
    assert!(
        !skill_path.exists(),
        "init should not install opencode skills without .opencode/ dir"
    );
}

#[test]
fn test_init_installs_repo_skill_docs() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".opencode")).unwrap();

    run_init_in(project.path(), home.path(), &["init"]);

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_skill = std::fs::read_to_string(repo.join(".opencode/skills/tala/SKILL.md")).unwrap();
    let installed =
        std::fs::read_to_string(project.path().join(".opencode/skills/tala/SKILL.md")).unwrap();
    assert_eq!(
        installed, repo_skill,
        "installed SKILL.md must be byte-identical to the repo file (single source of truth)"
    );
    assert!(
        !installed.contains("tala start"),
        "installed SKILL.md must not reference removed `tala start`"
    );
    assert!(
        !installed.contains("tala recap"),
        "installed SKILL.md must not reference removed `tala recap`"
    );

    let repo_cmd = repo.join(".opencode/commands/tala.md");
    let installed_cmd =
        std::fs::read_to_string(project.path().join(".opencode/commands/tala.md")).unwrap();
    assert_eq!(
        installed_cmd,
        std::fs::read_to_string(repo_cmd).unwrap(),
        "installed tala.md must match the repo file"
    );
}

/// Collect every `tala <command>` first-token referenced inside backtick spans and
/// fenced code blocks of a markdown doc. This mirrors how an agent would read the docs.
fn extract_doc_commands(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            // Whole line is code — scan every word pair.
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            for w in words.windows(2) {
                if w[0] == "tala" {
                    out.push(w[1].trim_start_matches('-').to_string());
                }
            }
        } else {
            // Inline backtick spans: odd-indexed segments after splitting on '`'.
            let parts: Vec<&str> = line.split('`').collect();
            for (i, seg) in parts.iter().enumerate() {
                if i % 2 == 1 {
                    let words: Vec<&str> = seg.split_whitespace().collect();
                    for w in words.windows(2) {
                        if w[0] == "tala" {
                            out.push(w[1].trim_start_matches('-').to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

#[test]
fn test_docs_reference_only_real_commands() {
    let home = tempfile::tempdir().unwrap();
    let help = tala_ok(home.path(), &["--help"]);

    // Parse the real command set from `tala --help`'s Commands: block.
    let mut commands = std::collections::BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.trim_start().starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            if line.trim().is_empty()
                || line.trim_start().starts_with("Options:")
                || line.trim_start().starts_with("Arguments:")
            {
                in_commands = false;
                continue;
            }
            if let Some(name) = line.split_whitespace().next() {
                if !name.starts_with('-') {
                    commands.insert(name.to_string());
                }
            }
        }
    }
    assert!(
        commands.contains("send") && commands.contains("session"),
        "help parsing should find known commands, got: {:?}",
        commands
    );

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        repo.join("README.md"),
        repo.join(".opencode/skills/tala/SKILL.md"),
        repo.join(".opencode/commands/tala.md"),
    ];

    // `tala-cli` (cargo binstall) and `tala <command>`-style placeholders are the only
    // non-subcommand tokens the docs legitimately contain.
    let allowlist = ["cli"];

    let mut failures: Vec<String> = Vec::new();
    for doc in &docs {
        let text =
            std::fs::read_to_string(doc).unwrap_or_else(|e| panic!("{}: {}", doc.display(), e));
        for token in extract_doc_commands(&text) {
            if token.is_empty() || token.starts_with('-') {
                continue;
            }
            if commands.contains(&token) || allowlist.contains(&token.as_str()) {
                continue;
            }
            failures.push(format!("{}: unknown `tala {}`", doc.display(), token));
        }
    }
    assert!(
        failures.is_empty(),
        "docs reference commands that do not exist in the binary:\n{}",
        failures.join("\n")
    );
}

#[test]
fn test_close_session() {
    let home = tempfile::tempdir().unwrap();

    let session = tala_start(home.path());

    let close = tala_ok(home.path(), &["close", &session]);
    assert!(close.contains("closed"), "close should confirm: {}", close);

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("closed"),
        "list should show session as closed"
    );

    tala_stop(home.path());
}

#[test]
fn test_agent_to_agent_conversation() {
    let home = tempfile::tempdir().unwrap();

    let session = tala_start(home.path());

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &session,
            "Bug in grubble: fix scope commits",
        ],
    );

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &session,
            "--sender",
            "grubble-agent",
            "Found it, fix pushed",
        ],
    );

    let recap = tala_ok(home.path(), &["history", &session]);
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
        "recap should attribute --sender name"
    );

    tala_stop(home.path());
}

#[test]
fn test_send_with_history() {
    let home = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "Starting message test"],
    );

    let history = tala_ok(home.path(), &["history", &sess]);
    assert!(history.contains("Starting message test"));

    tala_stop(home.path());
}

#[test]
fn test_wait_timeout() {
    let home = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["wait", &sess, "--timeout", "2"]);
    assert!(
        ok || stdout.contains("timeout"),
        "wait timeout should succeed or report timeout with code 2: {}",
        stdout
    );
    assert!(
        stdout.contains("timeout"),
        "wait should report timeout: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_since_returns_existing_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "existing-msg"]);

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["wait", &sess, "--since", "0", "--timeout", "3", "--json"],
    );
    assert!(ok, "wait --since should succeed");
    assert!(
        stdout.contains("existing-msg"),
        "should return existing msg"
    );
    assert!(stdout.contains("\"cursor\""), "should include cursor");

    tala_stop(home.path());
}

#[test]
fn test_wait_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "alpha", "msg-alpha"],
    );
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "beta", "msg-beta"],
    );

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &[
            "wait",
            &sess,
            "--since",
            "0",
            "--from",
            "alpha",
            "--timeout",
            "3",
            "--json",
        ],
    );
    assert!(ok, "wait --from should succeed");
    assert!(stdout.contains("alpha"), "should include alpha");
    assert!(!stdout.contains("beta"), "should exclude beta");

    tala_stop(home.path());
}

#[test]
fn test_wait_limit_cap() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "t", "m1"],
    );
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "t", "m2"],
    );
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "t", "m3"],
    );

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &[
            "wait",
            &sess,
            "--since",
            "0",
            "--from",
            "t",
            "--limit",
            "2",
            "--timeout",
            "3",
            "--json",
        ],
    );
    assert!(ok, "wait --limit should succeed");

    let count = stdout.matches("\"content\"").count();
    assert_eq!(count, 2, "should cap at 2 messages: {}", stdout);

    tala_stop(home.path());
}

#[test]
fn test_wait_limit_tail_four_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "m1"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m2"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m3"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m4"]);

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &[
            "wait",
            &sess,
            "--since",
            "0",
            "--limit",
            "2",
            "--timeout",
            "3",
            "--json",
        ],
    );
    assert!(ok, "wait --limit should succeed");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let contents: Vec<&str> = val["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["content"].as_str())
        .collect();
    assert_eq!(
        contents,
        vec!["m3", "m4"],
        "wait --limit 2 on 4 msgs should return NEWEST 2 (m3,m4): {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_returns_preexisting_incoming_session() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // alpha creates a session and asks a question BEFORE beta starts waiting
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let alpha_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &alpha_sess, "question-from-alpha"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // beta waits --new-session AFTER the session exists: must get alpha's question
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&alpha_sess),
        "should return alpha's session: {}",
        stdout
    );
    assert!(
        stdout.contains("question-from-alpha"),
        "should include alpha's message: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_ignores_own_session_create() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // Start the daemon and give alpha a session (empty, no message)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);

    // beta waits for an incoming session in the background
    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .current_dir(&beta_proj)
        .args(["wait", "--new-session", "--timeout", "15", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start wait --new-session");
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // beta creates its OWN session: the wait must NOT fire on it
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["session", "create"]);
    assert!(ok, "beta session create failed: {}", sout);
    let beta_sess = sout.trim().to_string();
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(
        child.try_wait().unwrap().is_none(),
        "wait --new-session must not fire on the waiter's own session create"
    );

    // alpha creates a NEW session and asks a question: the wait should fire now
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha second session create failed: {}", sout);
    let alpha_sess2 = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &alpha_sess2, "question-from-alpha"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "wait --new-session should exit 0: {}",
        stdout
    );
    assert!(
        stdout.contains(&alpha_sess2),
        "should return alpha's new session: {}",
        stdout
    );
    assert!(
        !stdout.contains(&beta_sess),
        "must not return beta's own session: {}",
        stdout
    );
    assert!(
        stdout.contains("question-from-alpha"),
        "should include the question: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_does_not_redeliver_consumed_message() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // alpha creates a session and sends a message (B029: consumed traffic must
    // never re-deliver on a later wait --new-session)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let alpha_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &alpha_sess, "consumed-question"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // beta reads the message (history advances beta's per-session cursor)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["history", &alpha_sess]);
    assert!(ok, "beta history failed: {}", sout);
    assert!(
        sout.contains("consumed-question"),
        "beta should see the message: {}",
        sout
    );

    // beta waits --new-session: the consumed message must NOT be re-delivered
    // (JSON mode reports the timeout in-band and exits 0)
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "4", "--json"],
    );
    assert!(ok, "wait --new-session should exit 0: {}", stdout);
    assert!(
        stdout.contains("\"timeout\":true"),
        "should report a timeout instead of re-delivering consumed msg: {}",
        stdout
    );

    // a genuinely NEW session from alpha must still be delivered
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha second create failed: {}", sout);
    let alpha_sess2 = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &alpha_sess2, "fresh-question"],
    );
    assert!(ok, "alpha second send failed: {}", sout);

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&alpha_sess2),
        "should return the new session: {}",
        stdout
    );
    assert!(
        stdout.contains("fresh-question"),
        "should include the new question: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_prefers_freshest_never_seen_session() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // OLDER session with an unread incoming message (stale backlog candidate)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha create failed: {}", sout);
    let stale_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &stale_sess, "stale-question"],
    );
    assert!(ok, "alpha send failed: {}", sout);
    std::thread::sleep(std::time::Duration::from_millis(600));

    // NEWER session with a fresh question (also never-seen by beta)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha create failed: {}", sout);
    let fresh_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &fresh_sess, "fresh-question"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // beta waits --new-session: must get the FRESH session, not the stale one
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&fresh_sess),
        "should return the freshest session: {}",
        stdout
    );
    assert!(
        !stdout.contains(&stale_sess),
        "must not return the stale session: {}",
        stdout
    );
    assert!(
        stdout.contains("fresh-question"),
        "should include the fresh question: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_excludes_waiter_created_session() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // beta creates a scratch session (cursor entry written on create); alpha
    // replies into it. The scratch session is beta's own — never a handshake.
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["session", "create"]);
    assert!(ok, "beta create failed: {}", sout);
    let beta_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &beta_sess, "reply-in-beta-scratch"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // beta waits --new-session: its own scratch session is excluded → timeout
    // (JSON mode reports the timeout in-band and exits 0)
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "4", "--json"],
    );
    assert!(ok, "wait --new-session should exit 0: {}", stdout);
    assert!(
        stdout.contains("\"timeout\":true"),
        "should report a timeout (own scratch session excluded): {}",
        stdout
    );

    // alpha's genuinely new session is then delivered
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha create failed: {}", sout);
    let fresh_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &fresh_sess, "fresh-question"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&fresh_sess),
        "should return alpha's new session: {}",
        stdout
    );
    assert!(
        !stdout.contains(&beta_sess),
        "must not return beta's own scratch session: {}",
        stdout
    );
    assert!(
        stdout.contains("fresh-question"),
        "should include the question: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_prefers_never_seen_over_seen_session() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // beta participates in S1 (cursor entry via create + send)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["session", "create"]);
    assert!(ok, "beta create failed: {}", sout);
    let seen_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["send", "--session", &seen_sess, "beta-note"],
    );
    assert!(ok, "beta send failed: {}", sout);

    // alpha creates a NEW session with a question (never-seen by beta)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha create failed: {}", sout);
    let fresh_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &fresh_sess, "fresh-question"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // alpha then sends a NEWER message into the SEEN session (S1): fresher
    // timestamp, but S1 is known to beta — never-seen S2 must still win
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &[
            "send",
            "--session",
            &seen_sess,
            "newer-reply-in-seen-session",
        ],
    );
    assert!(ok, "alpha send failed: {}", sout);

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&fresh_sess),
        "should return the never-seen session: {}",
        stdout
    );
    assert!(
        !stdout.contains(&seen_sess),
        "must not return the seen session even with a newer message: {}",
        stdout
    );
    assert!(
        stdout.contains("fresh-question"),
        "should include the question: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_recap_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "alpha",
            "only-alpha",
        ],
    );
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--sender", "beta", "only-beta"],
    );

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["history", &sess, "--json", "--from", "alpha"],
    );
    assert!(ok, "recap --from should succeed");
    assert!(stdout.contains("only-alpha"), "should include alpha msg");
    assert!(!stdout.contains("only-beta"), "should exclude beta msg");

    tala_stop(home.path());
}

#[test]
fn test_recap_cursor() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "old-msg"]);
    tala_ok(home.path(), &["send", "--session", &sess, "new-msg"]);

    let (stdout, _stderr, ok) = tala(home.path(), &["history", &sess, "--json", "--since", "1"]);
    assert!(ok, "recap --cursor should succeed");
    assert!(!stdout.contains("old-msg"), "should exclude old-msg");
    assert!(stdout.contains("new-msg"), "should include new-msg");

    tala_stop(home.path());
}

#[test]
fn test_recap_limit_cap() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "m1"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m2"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m3"]);

    let (stdout, _stderr, ok) = tala(home.path(), &["history", &sess, "--json", "--limit", "2"]);
    assert!(ok, "recap --limit should succeed");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let contents: Vec<&str> = val["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["content"].as_str())
        .collect();
    assert_eq!(
        contents,
        vec!["m2", "m3"],
        "limit 2 should return NEWEST 2 messages (m2,m3), not oldest: {}",
        stdout
    );
    assert_eq!(
        val["cursor"], 3,
        "cursor should be the max id returned (3): {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_recap_limit_tail_four_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "m1"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m2"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m3"]);
    tala_ok(home.path(), &["send", "--session", &sess, "m4"]);

    let (stdout, _stderr, ok) = tala(home.path(), &["history", &sess, "--json", "--limit", "2"]);
    assert!(ok, "recap --limit should succeed");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let contents: Vec<&str> = val["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["content"].as_str())
        .collect();
    assert_eq!(
        contents,
        vec!["m3", "m4"],
        "limit 2 on 4 msgs should return NEWEST 2 (m3,m4) in ascending order: {}",
        stdout
    );
    assert_eq!(
        val["cursor"], 4,
        "cursor should be max id returned (4): {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_recap_limit_zero_is_unlimited() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "a"]);
    tala_ok(home.path(), &["send", "--session", &sess, "b"]);
    tala_ok(home.path(), &["send", "--session", &sess, "c"]);

    let (stdout, _stderr, ok) = tala(home.path(), &["history", &sess, "--json", "--limit", "0"]);
    assert!(ok, "recap --limit 0 should succeed");
    let count = stdout.matches("\"content\"").count();
    assert!(
        count >= 3,
        "limit 0 should return all messages, got {}",
        count
    );

    tala_stop(home.path());
}

#[test]
fn test_send_json_output() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "--json", "json-test"],
    );
    assert!(ok, "send --json should succeed");
    assert!(stdout.contains("\"cursor\""), "should include cursor");
    assert!(stdout.contains("\"content\""), "should include content");

    tala_stop(home.path());
}

#[test]
fn test_close_json_output() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["close", &sess, "--json"]);
    assert!(ok, "close --json should succeed");
    assert!(stdout.contains("\"status\""), "should include status");
    assert!(stdout.contains("closed"), "status should be closed");

    tala_stop(home.path());
}

#[test]
fn test_status_json_output() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["status", "--json"]);
    assert!(ok, "status --json should succeed");
    assert!(stdout.contains("\"pid\""), "should include pid");

    tala_stop(home.path());
}

#[test]
fn test_list_json_output() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["list", "--json"]);
    assert!(ok, "list --json should succeed");
    assert!(
        stdout.contains("\"session_id\""),
        "should include session_id"
    );

    tala_stop(home.path());
}

#[test]
fn test_send_to_closed_session_fails() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);

    let (_stdout, stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "this should fail"],
    );
    assert!(!ok, "send to closed should fail");
    assert!(stderr.contains("closed"), "error should mention closed");

    tala_stop(home.path());
}

#[test]
fn test_send_json_nothing_to_send_is_json() {
    let home = tempfile::tempdir().unwrap();
    // One session exists (open) so the hint has something to point at.
    let sess = tala_start(home.path());
    // Fresh project dir with NO active session -> "Nothing to send" path.
    let project = tempfile::tempdir().unwrap();
    run_init_in(project.path(), home.path(), &["init"]);

    let (_stdout, stderr, ok) = tala_in(home.path(), Some(project.path()), &["send", "--json"]);
    assert!(!ok, "send --json with nothing to send should fail");
    assert!(
        !stderr.starts_with("Error:"),
        "--json error should not be the human block: {}",
        stderr
    );
    let v: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
    assert_eq!(
        v["code"], "NOTHING_TO_SEND",
        "code should be NOTHING_TO_SEND"
    );
    assert!(
        v["error"]
            .as_str()
            .unwrap_or("")
            .contains("Nothing to send"),
        "error should mention Nothing to send"
    );
    assert!(
        v["error"].as_str().unwrap_or("").contains(&sess),
        "hint should mention the open session id"
    );

    tala_stop(home.path());
}

#[test]
fn test_send_json_empty_message_is_json() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, stderr, ok) = tala(home.path(), &["send", "--session", &sess, "--json", ""]);
    assert!(!ok, "send --json empty message should fail");
    let v: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
    assert_eq!(v["code"], "EMPTY_MESSAGE", "code should be EMPTY_MESSAGE");

    tala_stop(home.path());
}

#[test]
fn test_send_hint_says_open_session_not_active() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let project = tempfile::tempdir().unwrap();
    run_init_in(project.path(), home.path(), &["init"]);

    let (_stdout, stderr, ok) = tala_in(home.path(), Some(project.path()), &["send"]);
    assert!(!ok, "bare send with nothing to send should fail");
    assert!(
        stderr.contains("Nothing to send"),
        "human error should mention Nothing to send"
    );
    assert!(
        stderr.contains(&format!("Open session: {}", sess)),
        "hint should say 'Open session:' with the open session id: {}",
        stderr
    );
    assert!(
        !stderr.contains("Active session:"),
        "hint must not mislabel an open session as active: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_send_json_closed_session_still_json() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    tala_ok(home.path(), &["close", &sess]);

    let (_stdout, stderr, ok) = tala(home.path(), &["send", "--session", &sess, "--json", "hi"]);
    assert!(!ok, "send to closed session should fail");
    let v: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
    assert_eq!(v["code"], "SESSION_CLOSED", "code should be SESSION_CLOSED");

    tala_stop(home.path());
}

#[test]
fn test_close_already_closed_fails() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);
    let (_stdout, _stderr, ok) = tala(home.path(), &["close", &sess]);
    assert!(!ok, "close already-closed should fail");

    tala_stop(home.path());
}

#[test]
fn test_wait_after_close_returns_messages_and_closed_true() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "pending-msg"]);
    tala_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["wait", &sess, "--since", "0", "--timeout", "3", "--json"],
    );
    assert!(ok, "wait after close should succeed");
    assert!(
        stdout.contains("\"closed\":true"),
        "should report closed:true"
    );
    assert!(
        stdout.contains("pending-msg"),
        "should return pending messages"
    );

    tala_stop(home.path());
}

#[test]
fn test_watch_after_close() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &[
            "stream",
            "--session",
            &sess,
            "--since",
            "0",
            "--timeout",
            "3",
            "--json",
        ],
    );
    assert!(ok, "stream after close should succeed");
    assert!(stdout.contains("closed"), "should emit closed event");

    tala_stop(home.path());
}

#[test]
fn test_empty_message_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["send", "--session", &sess, ""]);
    assert!(!ok, "empty message should be rejected");

    tala_stop(home.path());
}

#[test]
fn test_empty_session_name_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["session", "rename", &sess, ""]);
    assert!(!ok, "empty session name should be rejected");

    tala_stop(home.path());
}

#[test]
fn test_session_rename_and_show() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let close = tala_ok(
        home.path(),
        &["session", "rename", &sess, "my-project", "--force"],
    );
    assert!(close.contains("renamed"), "rename should confirm");

    let show = tala_ok(home.path(), &["session", "show", &sess]);
    assert!(show.contains("my-project"), "show should display name");

    tala_stop(home.path());
}

#[test]
fn test_session_close_alias() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let out = tala_ok(home.path(), &["session", "close", &sess]);
    assert!(out.contains("closed"), "session close should confirm");

    tala_stop(home.path());
}

#[test]
fn test_session_close_alias_clears_active() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    // two more open sessions so bare send hits the NO_ACTIVE_SESSION path
    // (with ≤1 open session left, send auto-targets instead of erroring)
    tala_start(home.path());
    tala_start(home.path());

    // Make sess the active session for this project dir
    let out = tala_in(home.path(), Some(project.path()), &["use", &sess]).0;
    assert!(
        out.contains("Active session"),
        "use should confirm: {}",
        out
    );

    // Close the ACTIVE session via the `session close` alias (same CWD as `use`,
    // since the active-session file is project-relative)
    let out = tala_in(
        home.path(),
        Some(project.path()),
        &["session", "close", &sess],
    )
    .0;
    assert!(out.contains("closed"), "session close should confirm");

    // The active-session marker must be cleared: `use` should list available
    // sessions instead of reporting the closed session as active
    let (stdout, _stderr, ok) = tala_in(home.path(), Some(project.path()), &["use"]);
    assert!(ok);
    assert!(
        stdout.contains("Available sessions"),
        "use should show available sessions, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Active session:"),
        "use should not report a stale active session: {}",
        stdout
    );

    // list must not show a * marker on the closed row
    let list = tala_ok(home.path(), &["list"]);
    assert!(
        !list.contains(" *"),
        "list should not show a * marker when active is cleared: {}",
        list
    );

    // Bare send must fail with a no-active-session error, not "Session is closed"
    let (_, stderr, ok) = tala_in(home.path(), Some(project.path()), &["send", "hello"]);
    assert!(!ok, "bare send after closing active should fail");
    assert!(
        stderr.contains("No active session"),
        "should mention no active session: {}",
        stderr
    );

    // list must not show a * marker on the closed row
    let list = tala_in(home.path(), Some(project.path()), &["list"]).0;
    assert!(
        !list.contains(" *"),
        "list should not show a * marker when active is cleared: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_close_explicit_id_clears_active() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    tala_start(home.path()); // keep an open session so `use` lists available

    let out = tala_in(home.path(), Some(project.path()), &["use", &sess]).0;
    assert!(
        out.contains("Active session"),
        "use should confirm: {}",
        out
    );

    // Close the ACTIVE session via top-level `close` with an explicit id
    let (stdout, stderr, ok) = tala_in(home.path(), Some(project.path()), &["close", &sess]);
    assert!(ok, "close should succeed: {} {}", stdout, stderr);
    assert!(
        stdout.contains("closed"),
        "close should confirm: {}",
        stdout
    );
    assert!(
        stderr.contains("cleared"),
        "close of active should mention clearing: {}",
        stderr
    );

    let (stdout, _stderr, ok) = tala_in(home.path(), Some(project.path()), &["use"]);
    assert!(ok);
    assert!(
        stdout.contains("Available sessions"),
        "use should show available sessions, got: {}",
        stdout
    );

    // list must not show a * marker on the closed row
    let list = tala_in(home.path(), Some(project.path()), &["list"]).0;
    assert!(
        !list.contains(" *"),
        "list should not show a * marker when active is cleared: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_close_non_active_keeps_active_marker() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess_a = tala_start(home.path());
    let sess_b = tala_start(home.path());

    let out = tala_in(home.path(), Some(project.path()), &["use", &sess_a]).0;
    assert!(
        out.contains("Active session"),
        "use should confirm: {}",
        out
    );

    // Close a NON-active session — the active marker must be untouched
    tala_in(
        home.path(),
        Some(project.path()),
        &["session", "close", &sess_b],
    );

    let out = tala_in(home.path(), Some(project.path()), &["use"]).0;
    assert!(
        out.contains(&sess_a),
        "active should still be sess_a: {}",
        out
    );
    assert!(
        !out.contains(&sess_b),
        "closed sess_b should not be active: {}",
        out
    );

    // Bare send should still target sess_a
    tala_in(home.path(), Some(project.path()), &["send", "still active"])
        .2
        .then_some(())
        .unwrap();
    let recap = tala_ok(home.path(), &["history", &sess_a]);
    assert!(
        recap.contains("still active"),
        "message should go to sess_a: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_session_close_alias_json_active_cleared() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    // one more open session so the closed one isn't the only session
    tala_start(home.path());

    tala_in(home.path(), Some(project.path()), &["use", &sess]);

    let out = tala_in(
        home.path(),
        Some(project.path()),
        &["session", "close", &sess, "--json"],
    )
    .0;
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        parsed["status"], "closed",
        "status should be closed: {}",
        out
    );
    assert_eq!(
        parsed["active_cleared"], true,
        "json should report active_cleared: {}",
        out
    );

    tala_stop(home.path());
}

#[test]
fn test_nonexistent_session_recap_fails() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["history", "nonexistent"]);
    assert!(!ok, "recap nonexistent should fail");

    tala_stop(home.path());
}

#[test]
fn test_nonexistent_session_wait_fails() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["wait", "nonexistent", "--timeout", "2"]);
    assert!(!ok, "wait nonexistent should fail");

    tala_stop(home.path());
}

#[test]
fn test_no_wait_flag_instead_of_ff() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "sent with --no-wait"],
    );
    assert!(ok, "--no-wait should work");
    assert!(
        stdout.contains("Sent message"),
        "should show confirmation: {}",
        stdout
    );

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(recap.contains("--no-wait"), "message should be in recap");

    tala_stop(home.path());
}

#[test]
fn test_send_short_no_wait() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "sent with -n short flag"],
    );
    assert!(ok, "-n should work");
    assert!(
        stdout.contains("Sent message"),
        "should show confirmation: {}",
        stdout
    );

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(recap.contains("-n short"), "message should be in recap");

    tala_stop(home.path());
}

#[test]
fn test_send_quiet_flag() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "--quiet", "quiet message"],
    );
    assert!(ok, "--quiet should still succeed");
    assert!(
        !stdout.contains("Sent"),
        "should not print confirmation: {:?}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_use_set_and_clear() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    // Set active session (in project dir so active-session is isolated)
    let out = tala_in(home.path(), Some(project.path()), &["use", &sess]).0;
    assert!(out.contains("Active session"), "should confirm: {}", out);

    // Show active session
    let out = tala_in(home.path(), Some(project.path()), &["use"]).0;
    assert!(out.contains(&sess), "should show session: {}", out);

    // Send without --session should use active session
    tala_in(
        home.path(),
        Some(project.path()),
        &["send", "sent via active session"],
    )
    .2
    .then_some(())
    .unwrap();

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("active session"),
        "message should be in session"
    );

    // Clear
    let out = tala_in(home.path(), Some(project.path()), &["use", "--clear"]).0;
    assert!(out.contains("cleared"), "should confirm clear: {}", out);

    // Verify cleared — should list available sessions (not active)
    let (stdout, _stderr, ok) = tala_in(home.path(), Some(project.path()), &["use"]);
    assert!(ok);
    assert!(
        stdout.contains("Available sessions"),
        "should list available sessions: {}",
        stdout
    );
    assert!(
        stdout.contains(&sess),
        "should show session in listing: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_use_json_output() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let out = tala_in(home.path(), Some(project.path()), &["use", &sess, "--json"]).0;
    assert!(
        out.contains("\"session_id\""),
        "json should have session_id: {}",
        out
    );
    assert!(out.contains(&sess), "json should contain session id");

    let out = tala_in(home.path(), Some(project.path()), &["use", "--json"]).0;
    assert!(
        out.contains("\"session_id\""),
        "json show should have session_id"
    );

    let out = tala_in(
        home.path(),
        Some(project.path()),
        &["use", "--clear", "--json"],
    )
    .0;
    assert!(out.contains("\"status\""), "json clear should have status");

    tala_stop(home.path());
}

#[test]
fn test_init_positional_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    run_init_in(project.path(), home.path(), &["init", "my-custom-project"]);

    let config_path = project.path().join(".tala").join("config.json");
    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config.contains("my-custom-project"),
        "positional name should be used: {}",
        config
    );
}

#[test]
fn test_init_name_conflict() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let (_stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["init", "positional-name", "--name", "flag-name"],
    );
    assert!(!ok, "both positional and --name should conflict");
}

#[test]
fn test_send_sets_active_session() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // Start session from project dir — sets active session there
    let stdout = tala_in(home.path(), Some(project.path()), &["session", "create"]).0;
    let sess = stdout.lines().next().unwrap_or("").trim().to_string();
    assert!(
        sess.starts_with("sess_"),
        "should return session ID from first line of: {}",
        stdout
    );

    // Send from same project dir (no --session needed, active session is set)
    tala_in(
        home.path(),
        Some(project.path()),
        &["send", "message via start"],
    )
    .2
    .then_some(())
    .unwrap();

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("message via start"),
        "message should reach session created by start: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_send_auto_creates_session() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let (stdout, _stderr, ok) =
        tala_in(home.path(), Some(project.path()), &["send", "auto-created"]);
    assert!(ok, "send without active session should auto-create");
    assert!(
        stdout.contains("sess_"),
        "should contain a session ID: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_send_no_active_session_with_existing_sessions_fails() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());

    // With a single open session, auto-target should succeed
    let (_stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "should auto-target"],
    );
    assert!(
        ok,
        "send with single open session should auto-target: stderr={}",
        stderr
    );

    // Verify message was delivered to the single session
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("should auto-target"),
        "recap should contain message from auto-target: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_send_auto_creates_json_output() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--json", "auto-created"],
    );
    assert!(ok, "send --json without active session should auto-create");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        val.get("session_id").and_then(|v| v.as_str()).is_some(),
        "JSON response should contain session_id: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_use_by_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    tala_ok(
        home.path(),
        &["session", "rename", &sess, "test-session", "--force"],
    );

    // Use by name from isolated project dir
    let out = tala_in(home.path(), Some(project.path()), &["use", "test-session"]).0;
    assert!(
        out.contains("Active session"),
        "use by name should confirm: {}",
        out
    );

    // Send should route to the named session
    tala_in(
        home.path(),
        Some(project.path()),
        &["send", "sent via name"],
    )
    .2
    .then_some(())
    .unwrap();
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("sent via name"),
        "message should reach the named session"
    );

    tala_stop(home.path());
}

#[test]
fn test_use_by_nonexistent_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (_stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["use", "nonexistent-name"],
    );
    assert!(!ok, "use by nonexistent name should fail");
    assert!(
        stderr.contains("nonexistent") || stderr.contains("No active"),
        "error should mention the name: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_list_shows_session_name() {
    let home = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    tala_ok(
        home.path(),
        &["session", "rename", &sess, "visible-name", "--force"],
    );

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("visible-name"),
        "list should show session name: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_timeout() {
    let home = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());

    let child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--json", "--timeout", "3"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "listen-timeout-test"],
    );

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "listen with timeout should exit successfully"
    );
    assert!(
        stdout.contains("listen-timeout-test"),
        "listen should capture the message: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_banner_text() {
    let home = tempfile::tempdir().unwrap();

    // No traffic: listen must still announce connection and close (B007).
    let output = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--timeout", "3"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "listen should exit 0: stdout={} stderr={}",
        stdout,
        stderr
    );
    assert!(
        stdout.contains("Listening on tala daemon"),
        "text listen should print a connection banner: {}",
        stdout
    );
    assert!(
        stdout.contains("connection closed"),
        "text listen should note the stream closed: {}",
        stdout
    );
    assert!(
        stdout.contains("(0 message(s))"),
        "closed note should carry the message count: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_banner_json_pure_stdout() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--json", "--timeout", "5"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(700));

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "banner-json-msg"],
    );

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // stdout must stay a pure JSON event stream (B007 channel discipline).
    assert!(
        !stdout.contains("Listening on"),
        "json listen stdout must not carry the banner: {}",
        stdout
    );
    assert!(
        stdout.contains("banner-json-msg"),
        "json listen should deliver the message: {}",
        stdout
    );
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "json listen stdout line must parse as JSON: {}",
            line
        );
    }
    assert!(
        stderr.contains("[listen] connected to tala daemon"),
        "json listen banner should go to stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("connection closed (1 message(s))"),
        "json listen closed note should carry the count on stderr: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_stream_banner() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let output = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args([
            "stream",
            "--session",
            &sess,
            "--since",
            "0",
            "--timeout",
            "3",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stream should exit 0: {}", stdout);
    assert!(
        stdout.contains("Streaming session") && stdout.contains(&sess),
        "stream should print a connection banner with the session id: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_streams_all_sessions() {
    let home = tempfile::tempdir().unwrap();
    let sess1 = tala_start(home.path());
    let sess2 = tala_start(home.path());

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess1,
            "--sender",
            "alpha",
            "listen-msg-1",
        ],
    );
    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess2,
            "--sender",
            "beta",
            "listen-msg-2",
        ],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("listen-msg-1"),
        "listen should see msg from session 1: {}",
        stdout
    );
    assert!(
        stdout.contains("listen-msg-2"),
        "listen should see msg from session 2: {}",
        stdout
    );
    assert!(stdout.contains("alpha"), "listen should show sender alpha");
    assert!(stdout.contains("beta"), "listen should show sender beta");

    tala_stop(home.path());
}

#[test]
fn test_listen_channel_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["session", "rename", &sess, "help:auth-module", "--force"],
    );

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--name", "help", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "helper",
            "help-request-msg",
        ],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("help-request-msg"),
        "listen --name should filter: {}",
        stdout
    );
    assert!(
        stdout.contains("help:auth-module"),
        "should include session name"
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--from", "monitor", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "monitor",
            "monitor-only-msg",
        ],
    );
    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "other",
            "should-be-filtered",
        ],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("monitor-only-msg"),
        "listen --from should include monitor msg: {}",
        stdout
    );
    assert!(
        !stdout.contains("should-be-filtered"),
        "listen --from should exclude other senders"
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_match_filter() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--match", "urgent", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "alert",
            "urgent: production issue",
        ],
    );
    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "chat",
            "just a normal update",
        ],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("urgent"),
        "listen --match should match urgent: {}",
        stdout
    );
    assert!(
        !stdout.contains("normal update"),
        "listen --match should exclude non-matching"
    );

    tala_stop(home.path());
}

#[test]
fn test_send_stdin() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["send", "--session", &sess, "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start tala");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"piped stdin message")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "stdin send should succeed");

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("piped stdin message"),
        "stdin message should be in recap"
    );

    tala_stop(home.path());
}

#[test]
fn test_stream_streams_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["stream", "--session", &sess, "--since", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start stream");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "streamer",
            "live-msg",
        ],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("live-msg"),
        "stream should stream msg: {}",
        stdout
    );
    assert!(stdout.contains("streamer"), "stream should show sender");

    tala_stop(home.path());
}

#[test]
fn test_stream_limit_caps_messages() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args([
            "stream",
            "--session",
            &sess,
            "--since",
            "0",
            "--limit",
            "1",
            "--json",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start stream");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(home.path(), &["send", "--session", &sess, "limit-1-a"]);
    tala_ok(home.path(), &["send", "--session", &sess, "limit-1-b"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.matches("\"content\"").count();
    assert_eq!(count, 1, "stream --limit 1 should cap at 1: {}", stdout);

    tala_stop(home.path());
}

#[test]
fn test_stream_limit_zero_is_unlimited() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args([
            "stream",
            "--session",
            &sess,
            "--since",
            "0",
            "--limit",
            "0",
            "--json",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start stream");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(home.path(), &["send", "--session", &sess, "unlim-a"]);
    tala_ok(home.path(), &["send", "--session", &sess, "unlim-b"]);

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unlim-a"), "limit 0 should show unlim-a");
    assert!(stdout.contains("unlim-b"), "limit 0 should show unlim-b");

    tala_stop(home.path());
}

#[test]
fn test_rename_succeeds_without_force() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["session", "rename", &sess, "original-name", "--force"],
    );

    let (_stdout, _stderr, ok) = tala(home.path(), &["session", "rename", &sess, "new-name"]);
    assert!(
        ok,
        "rename without --force should succeed when session has a name"
    );

    tala_stop(home.path());
}

#[test]
fn test_rename_noop_same_name() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["session", "rename", &sess, "test-name"]);

    let (_stdout, _stderr, ok) = tala(home.path(), &["session", "rename", &sess, "test-name"]);
    assert!(ok, "rename to same name without --force should succeed");

    tala_stop(home.path());
}

#[test]
fn test_stdin_flag_piped() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["send", "--session", &sess, "--stdin", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start tala");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"explicit stdin flag message")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "--stdin send should succeed");

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("explicit stdin flag message"),
        "stdin message with --stdin flag should be in recap"
    );

    tala_stop(home.path());
}

#[test]
fn test_session_reopen() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = tala(home.path(), &["session", "reopen", &sess]);
    assert!(ok, "reopen should succeed");
    assert!(
        stdout.contains("reopened"),
        "should mention reopened: {}",
        stdout
    );

    // Send to reopened session should work
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "post-reopen-msg"],
    );

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("post-reopen-msg"),
        "recap should show post-reopen message"
    );

    tala_stop(home.path());
}

#[test]
fn test_session_reopen_already_open() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["session", "reopen", &sess]);
    assert!(ok, "reopen on already open session should succeed");

    tala_stop(home.path());
}

#[test]
fn test_session_reopen_nonexistent() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(home.path(), &["session", "reopen", "nonexistent"]);
    assert!(!ok, "reopen nonexistent should fail");

    tala_stop(home.path());
}

#[test]
fn test_session_reopen_json() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, ok) = tala(home.path(), &["session", "reopen", &sess, "--json"]);
    assert!(ok, "reopen --json should succeed");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(val["status"], "reopened", "json status should be reopened");

    tala_stop(home.path());
}

#[test]
fn test_send_with_message() {
    let home = tempfile::tempdir().unwrap();

    let (stdout, _stderr, ok) = tala(home.path(), &["send", "delivery test"]);
    assert!(ok, "send should succeed");

    assert!(
        stdout.contains("Sent message"),
        "send should include confirmation: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_message_file_flag() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let msg_path = home.path().join("msg.txt");
    std::fs::write(&msg_path, "message via --message-file").unwrap();

    let (stdout, stderr, ok) = tala(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--message-file",
            msg_path.to_str().unwrap(),
        ],
    );
    assert!(ok, "--message-file should work");
    assert!(
        !stderr.contains("deprecated"),
        "should not show deprecation warning"
    );
    assert!(stdout.contains("Sent message"), "should show confirmation");

    let recap = tala_ok(home.path(), &["history", &sess, "--json"]);
    assert!(
        recap.contains("message via --message-file"),
        "file content should be in recap"
    );

    tala_stop(home.path());
}

#[test]
fn test_close_quiet() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["close", &sess, "--quiet"]);
    assert!(ok, "close --quiet should succeed");
    assert!(
        !stdout.contains("closed"),
        "quiet close should not print confirmation: '{}'",
        stdout
    );

    // Verify session is actually closed
    let list = tala_ok(home.path(), &["list"]);
    assert!(list.contains("closed"), "list should show closed");

    tala_stop(home.path());
}

#[test]
fn test_close_quiet_json() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (stdout, _stderr, ok) = tala(home.path(), &["close", &sess, "--quiet", "--json"]);
    assert!(ok, "close --quiet --json should succeed");
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(val["status"], "closed", "json should show closed status");

    tala_stop(home.path());
}

#[test]
fn test_use_on_closed_session_shows_reopen_hint() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["close", &sess]);

    let (_stdout, stderr, ok) = tala_in(home.path(), Some(project.path()), &["use", &sess]);
    assert!(!ok, "use on closed session should fail");
    assert!(
        stderr.contains("closed") && stderr.contains("reopen"),
        "error should mention closed and reopen: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_stream_alias_works() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    use std::process::{Command, Stdio};

    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args([
            "stream",
            "--session",
            &sess,
            "--since",
            "0",
            "--timeout",
            "3",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start stream");

    std::thread::sleep(std::time::Duration::from_millis(500));

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "stream-alias-test"],
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("stream-alias-test"),
        "stream alias should show messages: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_list_status_column_shows_open_not_active() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    // The status column must say "open" (not "active") for a non-closed session:
    // "active" is reserved for the selected session, shown via the `*` marker.
    let list = tala_in(home.path(), Some(project.path()), &["list"]).0;
    assert!(
        list.contains("open"),
        "list should show 'open' status column: {}",
        list
    );
    assert!(
        !list.contains("active"),
        "list status column must not use the word 'active' for open sessions: {}",
        list
    );

    // The selected session carries the `*` marker.
    tala_in(home.path(), Some(project.path()), &["use", &sess]);
    let list = tala_in(home.path(), Some(project.path()), &["list"]).0;
    let row = list
        .lines()
        .find(|l| l.contains(&sess))
        .unwrap_or_else(|| panic!("session row missing: {}", list));
    assert!(
        row.trim_end().ends_with('*'),
        "active session row should end with '*': {}",
        row
    );
    assert!(
        !list.contains("active"),
        "list must not contain 'active' even with a selected session: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_history_empty_session_shows_no_messages_note() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path()); // created empty via `session create`

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("no messages yet"),
        "history on an empty session should say so: {}",
        recap
    );

    // Even with a since far ahead of any message the note should appear.
    tala_ok(home.path(), &["send", "--session", &sess, "one message"]);
    let recap = tala_ok(home.path(), &["history", "--since", "999", &sess]);
    assert!(
        recap.contains("no messages yet"),
        "history with nothing in range should say so: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_reopen_does_not_change_active_session() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess_a = tala_start(home.path());
    let sess_b = tala_start(home.path());

    // Work in sess_a; sess_b is a closed session we only want to reopen.
    tala_in(home.path(), Some(project.path()), &["use", &sess_a]);
    tala_ok(home.path(), &["close", &sess_b]);

    let out = tala_ok(home.path(), &["session", "reopen", &sess_b]);
    assert!(out.contains("reopened"), "should mention reopened: {}", out);

    let active = tala_in(home.path(), Some(project.path()), &["use"]).0;
    assert!(
        active.contains(&sess_a),
        "active session must remain sess_a after reopening sess_b: {}",
        active
    );

    // And a bare send still targets sess_a (not the reopened sess_b).
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "still working in a"],
    );
    assert!(ok, "send via active session should succeed: {}", stdout);
    let recap = tala_ok(home.path(), &["history", &sess_a]);
    assert!(
        recap.contains("still working in a"),
        "message should land in sess_a: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]

fn test_daemon_restart_preserves_messages() {
    // B024: a daemon restart must not wipe the transcript. Session metadata
    // already survived; messages must now survive too (graceful stop path).
    let home = tempfile::tempdir().unwrap();

    let session = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &session, "message one"]);
    tala_ok(home.path(), &["send", "--session", &session, "message two"]);
    tala_ok(
        home.path(),
        &["send", "--session", &session, "message three"],
    );

    let before = tala_ok(home.path(), &["history", &session]);
    assert!(
        before.contains("message two"),
        "pre-restart history should contain the messages: {}",
        before
    );

    // Graceful stop, then any command auto-restarts the daemon.
    tala_stop(home.path());

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("3 msgs"),
        "message count must survive daemon restart: {}",
        list
    );

    let recap = tala_ok(home.path(), &["history", &session]);
    for needle in ["message one", "message two", "message three"] {
        assert!(
            recap.contains(needle),
            "history after restart should contain {:?}: {}",
            needle,
            recap
        );
    }
    let i1 = recap.find("message one").expect("msg one index");
    let i3 = recap.find("message three").expect("msg three index");
    assert!(i1 < i3, "transcript order must be preserved: {}", recap);

    tala_stop(home.path());
}

// ---- cycle-05: per-session read cursors (B014 / B023 / B025) ----

fn run_two_agents(home: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let alpha_proj = home.join("alpha-proj");
    let beta_proj = home.join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home, &["init", "alpha"]);
    run_init_in(&beta_proj, home, &["init", "beta"]);
    (alpha_proj, beta_proj)
}

fn line_for<'a>(out: &'a str, sess: &str) -> &'a str {
    out.lines().find(|l| l.contains(sess)).unwrap_or("")
}

#[test]
fn test_unread_is_per_session_new_session_msg_visible() {
    // B014: beta reads session A fully (its cursor for A = 3); alpha then sends
    // ONE message to a brand-new session B (per-session id 1 — below any
    // inflated global cursor). beta's `list` must show B with "(1 new)" and
    // `check` must report the message.
    let home = tempfile::tempdir().unwrap();
    let (alpha_proj, beta_proj) = run_two_agents(home.path());

    // alpha: session A with 3 messages (per-session ids 1..3)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let sess_a = sout.trim().to_string();
    for m in ["a-one", "a-two", "a-three"] {
        let (sout, _serr, ok) = tala_in(
            home.path(),
            Some(&alpha_proj),
            &["send", "--session", &sess_a, m],
        );
        assert!(ok, "alpha send failed: {}", sout);
    }

    // beta reads session A fully -> beta's cursor for A = 3
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["history", &sess_a]);
    assert!(ok, "beta history failed: {}", sout);
    assert!(
        sout.contains("a-three"),
        "beta should see all of A: {}",
        sout
    );

    // alpha: brand-new session B + 1 message (per-session id 1)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha second session create failed: {}", sout);
    let sess_b = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess_b, "fresh-msg-for-beta"],
    );
    assert!(ok, "alpha send to B failed: {}", sout);

    // beta: list must show B as "(1 new)" and A as fully read
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["list"]);
    assert!(ok, "beta list failed: {}", sout);
    let a_line = line_for(&sout, &sess_a);
    let b_line = line_for(&sout, &sess_b);
    assert!(
        !a_line.contains("(1 new)"),
        "session A must not show unread after being read: {}",
        a_line
    );
    assert!(
        b_line.contains("(1 new)"),
        "session B must show (1 new) for its fresh message: {}",
        b_line
    );

    // beta: check --json must report the fresh message and a per-session cursors map
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["check", "--json"]);
    assert!(ok, "beta check failed: {}", sout);
    assert!(
        sout.contains("fresh-msg-for-beta"),
        "check must report B's fresh message: {}",
        sout
    );
    assert!(
        sout.contains("\"cursors\""),
        "check JSON must include cursors map: {}",
        sout
    );
    assert!(
        sout.contains(&format!("\"{}\":1", sess_b)),
        "cursors map must track B at 1: {}",
        sout
    );

    tala_stop(home.path());
}

#[test]
fn test_empty_session_history_does_not_reset_other_sessions_read_state() {
    // B025: reading an EMPTY session must not touch any other session's read
    // state (the old global cursor was reset to 0 by `history` on empty
    // sessions, re-marking everything unread).
    let home = tempfile::tempdir().unwrap();
    let (alpha_proj, beta_proj) = run_two_agents(home.path());

    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let sess_a = sout.trim().to_string();
    for m in ["a-one", "a-two", "a-three"] {
        let _ = tala_in(
            home.path(),
            Some(&alpha_proj),
            &["send", "--session", &sess_a, m],
        );
    }

    // beta reads A fully
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["history", &sess_a]);
    assert!(ok, "beta history failed: {}", sout);

    // alpha creates an EMPTY session (no message)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha empty session create failed: {}", sout);
    let sess_empty = sout.trim().to_string();

    // beta reads the empty session -> must not reset anything
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["history", &sess_empty]);
    assert!(ok, "beta history on empty session failed: {}", sout);
    assert!(
        sout.contains("(no messages yet)"),
        "empty history should say so: {}",
        sout
    );

    // A must still show no unread for beta
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["list"]);
    assert!(ok, "beta list failed: {}", sout);
    let a_line = line_for(&sout, &sess_a);
    assert!(
        !a_line.contains("(1 new)"),
        "reading an empty session must not re-mark A as unread: {}",
        a_line
    );

    // and the persisted cursors map must still track A at 3
    let cursors_file = beta_proj.join(".tala").join("cursors.json");
    let content = std::fs::read_to_string(&cursors_file)
        .unwrap_or_else(|e| panic!("cursors.json should exist: {}", e));
    assert!(
        content.contains(&format!("\"{}\":3", sess_a)),
        "cursors.json must keep A's read marker at 3: {}",
        content
    );


    tala_stop(home.path());
}

#[test]
fn test_message_ids_resume_after_restart() {
    // Per-session ids must continue after a restart — no reuse, no gaps.
    let home = tempfile::tempdir().unwrap();
    let session = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &session, "one"]);
    tala_ok(home.path(), &["send", "--session", &session, "two"]);
    tala_ok(home.path(), &["send", "--session", &session, "three"]);

    tala_stop(home.path());

    let out = tala_ok(home.path(), &["send", "--session", &session, "four"]);
    assert!(
        out.contains("message 4"),
        "next message id must resume at 4 after restart: {}",
        out
    );

    let recap = tala_ok(home.path(), &["history", &session]);
    assert!(
        recap.contains("[4]"),
        "history should show the resumed id 4: {}",
        recap
    );
    tala_stop(home.path());
}

#[test]
fn test_daemon_restart_preserves_session_names() {
    let home = tempfile::tempdir().unwrap();
    let session = tala_start(home.path());
    tala_ok(
        home.path(),
        &["session", "rename", &session, "durable-name"],
    );

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("durable-name"),
        "rename should apply before restart: {}",
        list
    );

    tala_stop(home.path());

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("durable-name"),
        "session name must survive daemon restart: {}",
        list
    );
    tala_stop(home.path());
}

#[test]
fn test_daemon_crash_after_rename_keeps_sessions() {
    // B027: `session rename` used to write sessions.json in a legacy name-only
    // format that load_sessions cannot parse; a crash (SIGKILL) before any
    // graceful persist therefore lost ALL session metadata on restart.
    let home = tempfile::tempdir().unwrap();
    let session = tala_start(home.path());
    tala_ok(
        home.path(),
        &["session", "rename", &session, "crash-durable"],
    );

    // Hard-kill the daemon (no graceful shutdown).
    let daemon_json = home.path().join(".tala").join("daemon.json");
    let info: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&daemon_json).unwrap()).unwrap();
    let pid = info["pid"].as_u64().unwrap() as i32;
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Next command must auto-restart and still see the session + its name.
    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains(&session),
        "session must survive crash-after-rename: {}",
        list
    );
    assert!(
        list.contains("crash-durable"),
        "renamed session must survive crash-after-rename: {}",
        list
    );
    tala_stop(home.path());
}

#[test]
fn test_daemon_restart_with_corrupt_messages_file() {
    // Backward compat / resilience: a corrupt messages.json must not prevent
    // the daemon from starting (degrades to empty transcript, sessions intact).
    let home = tempfile::tempdir().unwrap();
    let session = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &session, "pre-corrupt"]);

    tala_stop(home.path());

    let messages_path = home.path().join(".tala").join("messages.json");
    assert!(
        messages_path.exists(),
        "messages.json should be persisted on send"
    );
    std::fs::write(&messages_path, "{not valid json").unwrap();

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains(&session),
        "sessions must survive a corrupt messages.json: {}",
        list
    );
    tala_stop(home.path());
}

#[test]
fn test_send_to_session_without_messages_after_restart() {
    // Regression: after a restart, sending to a session that has no persisted
    // messages must work (add_message needs a next-id entry even when the
    // session has an empty transcript).
    let home = tempfile::tempdir().unwrap();
    let session = tala_start(home.path()); // created without a message

    tala_stop(home.path());
    tala_ok(home.path(), &["list"]); // restart

    let out = tala_ok(
        home.path(),
        &["send", "--session", &session, "first after restart"],
    );
    assert!(
        out.contains("message 1"),
        "first send after restart to an empty session should get id 1: {}",
        out
    );

    let recap = tala_ok(home.path(), &["history", &session]);
    assert!(
        recap.contains("first after restart"),
        "history should show the message: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_send_to_one_session_does_not_hide_unread_elsewhere() {
    // B023: sending a message writes the SENDER's cursor for THAT session only.
    // It must not inflate any global state that hides unread in other sessions.
    let home = tempfile::tempdir().unwrap();
    let (alpha_proj, beta_proj) = run_two_agents(home.path());

    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let sess_a = sout.trim().to_string();
    for m in ["a-one", "a-two", "a-three"] {
        let _ = tala_in(
            home.path(),
            Some(&alpha_proj),
            &["send", "--session", &sess_a, m],
        );
    }
    // beta reads A (cursor A = 3)
    let _ = tala_in(home.path(), Some(&beta_proj), &["history", &sess_a]);

    // alpha: new session B with 1 message (per-session id 1)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session B create failed: {}", sout);
    let sess_b = sout.trim().to_string();
    let _ = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess_b, "fresh-msg-for-beta"],
    );

    // beta SENDS a message into session A (per-session id 4): under the old
    // model this wrote 4 into the GLOBAL cursor and hid B's msg (id 1 <= 4).
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["send", "--session", &sess_a, "beta reply in a"],
    );
    assert!(ok, "beta send failed: {}", sout);

    // B must STILL show (1 new)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["list"]);
    assert!(ok, "beta list failed: {}", sout);
    let b_line = line_for(&sout, &sess_b);
    assert!(
        b_line.contains("(1 new)"),
        "sending in A must not hide B's unread: {}",
        b_line
    );
    // and check must still report B's message
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["check", "--json"]);
    assert!(ok, "beta check failed: {}", sout);
    assert!(
        sout.contains("fresh-msg-for-beta"),
        "check must still report B's message: {}",
        sout
    );

    tala_stop(home.path());
}

#[test]
fn test_check_updates_per_session_cursors_then_reports_nothing_new() {
    let home = tempfile::tempdir().unwrap();
    let (alpha_proj, beta_proj) = run_two_agents(home.path());

    // alpha: two sessions, A with 3 msgs, B with 1 msg
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "create A failed: {}", sout);
    let sess_a = sout.trim().to_string();
    for m in ["a-one", "a-two", "a-three"] {
        let _ = tala_in(
            home.path(),
            Some(&alpha_proj),
            &["send", "--session", &sess_a, m],
        );
    }
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "create B failed: {}", sout);
    let sess_b = sout.trim().to_string();
    let _ = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess_b, "b-one"],
    );

    // beta: first check reports both sessions with a per-session cursors map
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["check", "--json"]);
    assert!(ok, "first check failed: {}", sout);
    assert!(
        sout.contains("a-one"),
        "first check should show A msgs: {}",
        sout
    );
    assert!(
        sout.contains("b-one"),
        "first check should show B msgs: {}",
        sout
    );
    assert!(
        sout.contains(&format!("\"{}\":3", sess_a)),
        "cursors map must track A at 3: {}",
        sout
    );
    assert!(
        sout.contains(&format!("\"{}\":1", sess_b)),
        "cursors map must track B at 1: {}",
        sout
    );

    // beta: second check reports nothing new
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["check", "--json"]);
    assert!(ok, "second check failed: {}", sout);
    assert!(
        !sout.contains("a-one") && !sout.contains("b-one"),
        "second check must report nothing new: {}",
        sout
    );

    tala_stop(home.path());
}

#[test]
fn test_listen_replays_new_session_message_without_since() {
    // B014-for-listen: with no --since, `listen` must replay a NEW session's
    // message even though its per-session id (1) is below another session's
    // read cursor. The old default (single global cursor) skipped it.
    let home = tempfile::tempdir().unwrap();
    let (alpha_proj, beta_proj) = run_two_agents(home.path());

    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "create A failed: {}", sout);
    let sess_a = sout.trim().to_string();
    let _ = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess_a, "old-a-msg"],
    );
    // beta reads A -> beta's cursor for A = 1
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["history", &sess_a]);
    assert!(ok, "beta history failed: {}", sout);

    // alpha: new session B + message id 1 (below A's cursor but a different session)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "create B failed: {}", sout);
    let sess_b = sout.trim().to_string();
    let _ = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess_b, "fresh-listen-msg"],
    );

    // beta: listen with no --since must replay B's message but not A's read ones
    let child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .current_dir(&beta_proj)
        .args(["listen", "--timeout", "3"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "listen should exit 0: {}", stdout);
    assert!(
        stdout.contains("fresh-listen-msg"),
        "listen must replay the new session's message: {}",
        stdout
    );
    assert!(
        !stdout.contains("old-a-msg"),
        "listen must not replay session A (already read): {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_timeout_exits_3() {
    // B011/B018: a benign wait timeout must exit with the dedicated code 3,
    // not clap's usage-error code 2.
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let out = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["wait", &sess, "--timeout", "2"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "wait <sess> timeout should exit 3, got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // JSON variant too.
    let out = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["wait", &sess, "--timeout", "2", "--json"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "wait <sess> --json timeout should exit 3, got {:?}",
        out.status.code()
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("timeout"),
        "json output should report the timeout"
    );

    tala_stop(home.path());
}

#[test]
fn test_send_wait_timeout_exits_3() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let out = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["send", &sess, "ping", "--wait", "--timeout", "2"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "send --wait timeout should exit 3, got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_session_timeout_exits_3() {
    let home = tempfile::tempdir().unwrap();
    let _sess = tala_start(home.path());

    let out = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["wait", "--new-session", "--timeout", "2"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "wait --new-session timeout should exit 3, got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    tala_stop(home.path());
}

