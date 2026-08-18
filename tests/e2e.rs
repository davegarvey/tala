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

const TALA_SKILL_MIN_VERSION: &str = "0.27.3";
const TALA_CLI_MIN_VERSION_PLACEHOLDER: &str = "__TALA_CLI_MIN_VERSION__";
const TALA_CLI_GENERATED_VERSION_PLACEHOLDER: &str = "__TALA_CLI_GENERATED_VERSION__";

fn render_embedded_document(template: &str) -> String {
    template
        .replace(TALA_CLI_MIN_VERSION_PLACEHOLDER, TALA_SKILL_MIN_VERSION)
        .replace(
            TALA_CLI_GENERATED_VERSION_PLACEHOLDER,
            env!("CARGO_PKG_VERSION"),
        )
}

fn assert_versioned_document(document: &str, includes_skill_version: bool) {
    assert!(
        document.contains(&format!(
            "tala_cli_min_version: \"{}\"",
            TALA_SKILL_MIN_VERSION
        )),
        "document should contain the CLI minimum version: {}",
        document
    );
    assert!(
        document.contains(&format!(
            "tala_cli_generated_version: \"{}\"",
            env!("CARGO_PKG_VERSION")
        )),
        "document should contain the generating CLI version: {}",
        document
    );
    if includes_skill_version {
        assert!(
            document.contains("version: \"3.1\""),
            "skill should retain its independent content version"
        );
    }
}

fn assert_versioned_template(template: &str, includes_skill_version: bool) {
    assert_eq!(
        template.matches(TALA_CLI_MIN_VERSION_PLACEHOLDER).count(),
        1,
        "template should contain one CLI minimum placeholder"
    );
    assert_eq!(
        template
            .matches(TALA_CLI_GENERATED_VERSION_PLACEHOLDER)
            .count(),
        1,
        "template should contain one generated-version placeholder"
    );
    if includes_skill_version {
        assert!(
            template.contains("version: \"3.1\""),
            "skill template should retain its independent content version"
        );
        assert!(
            template.contains("tala --version") && template.contains("Semantic Versioning 2.0.0"),
            "skill template should document CLI compatibility checks"
        );
    }
}

// Creates a session in a named project dir and returns (session_id, project_dir)
fn tala_start_in(home: &std::path::Path, name: &str) -> (String, std::path::PathBuf) {
    let project = home.join(name);
    std::fs::create_dir_all(&project).unwrap();
    let (stdout, stderr, ok) = tala_in(home, Some(&project), &["session", "create"]);
    assert!(ok, "session create failed: {} {}", stdout, stderr);
    let sess = stdout.lines().next().unwrap_or("").trim().to_string();
    (sess, project)
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
fn test_init_preserves_existing_config() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let tala_dir = project.path().join(".tala");
    std::fs::create_dir_all(&tala_dir).unwrap();
    let config_path = tala_dir.join("config.json");
    std::fs::write(&config_path, "{\"name\":\"existing-agent\"}").unwrap();

    let (_stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["init", "replacement-agent"],
    );
    assert!(ok, "init should preserve an existing config: {}", stderr);
    assert_eq!(
        std::fs::read_to_string(config_path).unwrap(),
        "{\"name\":\"existing-agent\"}"
    );
    assert!(stderr.contains("already exists"));
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
    assert_versioned_document(&skill, true);

    let command_path = project
        .path()
        .join(".opencode")
        .join("commands")
        .join("tala.md");
    assert!(
        command_path.exists(),
        "init should detect .opencode/ and create command file at .opencode/commands/tala.md"
    );
    let command = std::fs::read_to_string(&command_path).unwrap();
    assert_versioned_document(&command, false);
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
        installed,
        render_embedded_document(&repo_skill),
        "installed SKILL.md must equal the rendered repo template"
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
        render_embedded_document(&std::fs::read_to_string(repo_cmd).unwrap()),
        "installed tala.md must equal the rendered repo template"
    );
    assert_versioned_document(&installed, true);
    assert_versioned_document(&installed_cmd, false);
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

    let skill_template = std::fs::read_to_string(&docs[1]).unwrap();
    let command_template = std::fs::read_to_string(&docs[2]).unwrap();
    assert_versioned_template(&skill_template, true);
    assert_versioned_template(&command_template, false);

    // `tala-cli` (cargo binstall) and `tala <command>`-style placeholders are the only
    // non-subcommand tokens the docs legitimately contain.
    let allowlist = ["cli", "version", "<version>"];

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

fn git_init(directory: &std::path::Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(directory)
        .status()
        .unwrap();
    assert!(status.success(), "git init should succeed");
}

fn init_json(
    home: &std::path::Path,
    project: &std::path::Path,
    args: &[&str],
) -> serde_json::Value {
    let (stdout, stderr, ok) = tala_in(home, Some(project), args);
    assert!(
        ok,
        "tala {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        stdout,
        stderr
    );
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "tala {} should emit JSON: {}\nstdout: {}\nstderr: {}",
            args.join(" "),
            error,
            stdout,
            stderr
        )
    })
}

#[test]
fn test_init_help_lists_safety_controls_and_rejects_unknown_flags() {
    let home = tempfile::tempdir().unwrap();
    let (stdout, stderr, ok) = tala_in(home.path(), None, &["init", "--help"]);
    assert!(ok, "init help failed: {}", stderr);
    for flag in ["--dry-run", "--force", "--gitignore", "--json"] {
        assert!(stdout.contains(flag), "init help should list {}", flag);
    }

    let project = tempfile::tempdir().unwrap();
    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["init", "--not-an-init-flag"],
    );
    assert!(!ok, "unknown init flags must fail");
    assert!(
        stdout.contains("unexpected argument") || stderr.contains("unexpected argument"),
        "unknown init flag should be reported: {} {}",
        stdout,
        stderr
    );
}

#[test]
fn test_init_repeat_skips_changed_file_and_force_replaces_it() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opencode")).unwrap();
    run_init_in(project.path(), home.path(), &["init", "safe-agent"]);

    let config_path = project.path().join(".tala/config.json");
    let original_config = std::fs::read_to_string(&config_path).unwrap();
    let skill_path = project.path().join(".opencode/skills/tala/SKILL.md");
    std::fs::write(&skill_path, "local agent customization\n").unwrap();

    let report = init_json(home.path(), project.path(), &["init", "--json"]);
    let skill = report["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"].as_str().unwrap().ends_with("SKILL.md"))
        .unwrap();
    assert_eq!(skill["action"], "skipped");
    assert!(report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("--force")));
    assert_eq!(
        std::fs::read_to_string(&skill_path).unwrap(),
        "local agent customization\n"
    );

    let forced = init_json(home.path(), project.path(), &["init", "--force", "--json"]);
    let forced_skill = forced["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"].as_str().unwrap().ends_with("SKILL.md"))
        .unwrap();
    assert_eq!(forced_skill["action"], "overwritten");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let expected = render_embedded_document(
        &std::fs::read_to_string(repo.join(".opencode/skills/tala/SKILL.md")).unwrap(),
    );
    assert_eq!(std::fs::read_to_string(skill_path).unwrap(), expected);
    assert_eq!(
        std::fs::read_to_string(config_path).unwrap(),
        original_config
    );
}

#[test]
fn test_init_dry_run_reports_without_writing() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opencode")).unwrap();
    git_init(project.path());

    let report = init_json(
        home.path(),
        project.path(),
        &["init", "--dry-run", "--gitignore", "--json"],
    );
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["config"]["action"], "would_create");
    assert!(report["files"].as_array().unwrap().iter().all(|file| {
        matches!(
            file["action"].as_str(),
            Some("would_create") | Some("would_unchanged") | Some("would_skip")
        )
    }));
    assert_eq!(report["gitignore"]["action"], "would_add");
    assert!(!project.path().join(".tala").exists());
    assert!(!project.path().join(".gitignore").exists());
    assert!(!project
        .path()
        .join(".opencode/skills/tala/SKILL.md")
        .exists());
}

#[test]
fn test_init_gitignore_is_opt_in_and_idempotent() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    git_init(project.path());

    run_init_in(project.path(), home.path(), &["init"]);
    assert!(!project.path().join(".gitignore").exists());

    let added = init_json(
        home.path(),
        project.path(),
        &["init", "--gitignore", "--json"],
    );
    assert_eq!(added["gitignore"]["action"], "added");
    assert_eq!(
        std::fs::read_to_string(project.path().join(".gitignore")).unwrap(),
        "/.tala/\n"
    );

    let present = init_json(
        home.path(),
        project.path(),
        &["init", "--gitignore", "--json"],
    );
    assert_eq!(present["gitignore"]["action"], "present");
    assert_eq!(
        std::fs::read_to_string(project.path().join(".gitignore")).unwrap(),
        "/.tala/\n"
    );
}

#[test]
fn test_init_gitignore_uses_nested_root_and_warns_outside_git() {
    let home = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    git_init(repository.path());
    let nested = repository.path().join("nested/project");
    std::fs::create_dir_all(&nested).unwrap();

    let nested_report = init_json(home.path(), &nested, &["init", "--gitignore", "--json"]);
    assert_eq!(nested_report["gitignore"]["action"], "added");
    assert!(repository.path().join(".gitignore").exists());
    assert!(!nested.join(".gitignore").exists());

    let outside = tempfile::tempdir().unwrap();
    let outside_report = init_json(
        home.path(),
        outside.path(),
        &["init", "--gitignore", "--json"],
    );
    assert_eq!(outside_report["gitignore"]["action"], "unavailable");
    assert!(!outside.path().join(".gitignore").exists());
    assert!(outside_report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("Git repository")));
}

#[test]
fn test_version_compatibility_guidance_handles_legacy_docs() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(repo.join(".opencode/skills/tala/SKILL.md")).unwrap();
    let legacy_skill = "---\nname: tala\nmetadata:\n  version: \"3.0\"\n---\n";

    assert!(!legacy_skill.contains("tala_cli_min_version"));
    assert!(!legacy_skill.contains("tala_cli_generated_version"));
    assert!(skill.contains("unversioned"));
    assert!(skill.contains("tala --help"));
    assert!(skill.contains("tala init"));
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
    let grubble_proj = init_project(home.path(), "grubble-agent");

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

    // Honest second-agent pattern: send from grubble-agent's own project dir
    // (B004 restrict: --sender may no longer fake another identity).
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(grubble_proj.path()),
        &["send", "--session", &session, "Found it, fix pushed"],
    );
    assert!(ok, "grubble-agent send failed: {sout} {serr}");

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
    let alpha_proj = init_project(home.path(), "alpha");
    let beta_proj = init_project(home.path(), "beta");
    let sess = tala_start(home.path());

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(alpha_proj.path()),
        &["send", "--session", &sess, "msg-alpha"],
    );
    assert!(ok, "alpha send failed: {sout} {serr}");
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(beta_proj.path()),
        &["send", "--session", &sess, "msg-beta"],
    );
    assert!(ok, "beta send failed: {sout} {serr}");

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
    let t_proj = init_project(home.path(), "t");
    let sess = tala_start(home.path());

    for m in ["m1", "m2", "m3"] {
        let (sout, serr, ok) = tala_in(
            home.path(),
            Some(t_proj.path()),
            &["send", "--session", &sess, m],
        );
        assert!(ok, "send {m} failed: {sout} {serr}");
    }

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

    let count = stdout.matches("\"parts\"").count();
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
fn test_wait_new_returns_seen_session_with_new_unread() {
    // B029 follow-up (v6 rerun finding): when NO never-seen session exists,
    // wait --new-session falls back to known sessions with UNREAD incoming
    // messages from another agent (id > the waiter's cursor) — the same event
    // the live loop fires on.
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // beta participates in S1 (create + send => cursor entry)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["session", "create"]);
    assert!(ok, "beta create failed: {}", sout);
    let sess = sout.trim().to_string();
    tala_in(
        home.path(),
        Some(&beta_proj),
        &["send", "--session", &sess, "beta-note"],
    );

    // alpha replies into the KNOWN session after beta's last message
    tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess, "alpha-follow-up"],
    );

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session should succeed: {}", stdout);
    assert!(
        stdout.contains(&sess),
        "known session with unread incoming must be returned: {}",
        stdout
    );
    assert!(
        stdout.contains("alpha-follow-up"),
        "should include the unread message: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_new_seen_session_without_unread_never_returns() {
    // B029 symptom 2 guard: fully-read known sessions are never re-delivered
    // by the scan (no re-looping on consumed traffic).
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    // beta participates in S1 and READ everything (history advances the cursor)
    let (sout, _serr, ok) = tala_in(home.path(), Some(&beta_proj), &["session", "create"]);
    assert!(ok, "beta create failed: {}", sout);
    let sess = sout.trim().to_string();
    tala_in(
        home.path(),
        Some(&beta_proj),
        &["send", "--session", &sess, "beta-note"],
    );
    tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess, "alpha-answer"],
    );
    tala_in(
        home.path(),
        Some(&beta_proj),
        &["history", "--session", &sess],
    );

    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "2", "--json"],
    );
    assert!(ok, "wait --new-session timeout exits 2: {}", stdout);
    assert!(
        !stdout.contains(&sess),
        "fully-read session must not be re-delivered: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_recap_from_filter() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = init_project(home.path(), "alpha");
    let beta_proj = init_project(home.path(), "beta");
    let sess = tala_start(home.path());

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(alpha_proj.path()),
        &["send", "--session", &sess, "only-alpha"],
    );
    assert!(ok, "alpha send failed: {sout} {serr}");
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(beta_proj.path()),
        &["send", "--session", &sess, "only-beta"],
    );
    assert!(ok, "beta send failed: {sout} {serr}");

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
fn test_session_rename() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let close = tala_ok(
        home.path(),
        &["session", "rename", &sess, "my-project", "--force"],
    );
    assert!(close.contains("renamed"), "rename should confirm");

    let listed = tala_ok(home.path(), &["list"]);
    assert!(listed.contains("my-project"), "list should display name");

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
        "listen that captured a message should exit 0"
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

    assert_eq!(
        output.status.code(),
        Some(3),
        "listen timeout should exit 3 (family contract): stdout={} stderr={}",
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
fn test_listen_streams_all_sessions() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = init_project(home.path(), "alpha");
    let beta_proj = init_project(home.path(), "beta");
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

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(alpha_proj.path()),
        &["send", "--session", &sess1, "listen-msg-1"],
    );
    assert!(ok, "alpha send failed: {sout} {serr}");
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(beta_proj.path()),
        &["send", "--session", &sess2, "listen-msg-2"],
    );
    assert!(ok, "beta send failed: {sout} {serr}");

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
    let helper_proj = init_project(home.path(), "helper");
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

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(helper_proj.path()),
        &["send", "--session", &sess, "help-request-msg"],
    );
    assert!(ok, "helper send failed: {sout} {serr}");

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
    let monitor_proj = init_project(home.path(), "monitor");
    let other_proj = init_project(home.path(), "other");
    let sess = tala_start(home.path());

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--from", "monitor", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(monitor_proj.path()),
        &["send", "--session", &sess, "monitor-only-msg"],
    );
    assert!(ok, "monitor send failed: {sout} {serr}");
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(other_proj.path()),
        &["send", "--session", &sess, "should-be-filtered"],
    );
    assert!(ok, "other send failed: {sout} {serr}");

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
    let alert_proj = init_project(home.path(), "alert");
    let chat_proj = init_project(home.path(), "chat");
    let sess = tala_start(home.path());

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--since", "0", "--match", "urgent", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(500));

    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(alert_proj.path()),
        &["send", "--session", &sess, "urgent: production issue"],
    );
    assert!(ok, "alert send failed: {sout} {serr}");
    let (sout, serr, ok) = tala_in(
        home.path(),
        Some(chat_proj.path()),
        &["send", "--session", &sess, "just a normal update"],
    );
    assert!(ok, "chat send failed: {sout} {serr}");

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
fn test_send_intent_badges() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &["send", "--session", &sess, "--intent", "req", "question"],
    );
    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "reply",
            "--reply-to",
            "1",
            "answer",
        ],
    );
    tala_ok(home.path(), &["send", "--session", &sess, "plain-status"]);

    let (stdout, _stderr, ok) = tala(home.path(), &["history", "--session", &sess]);
    assert!(ok, "history should succeed");
    assert!(
        stdout.contains("[REQ]"),
        "history shows req badge: {}",
        stdout
    );
    assert!(
        stdout.contains("[REPLY→1]"),
        "history shows correlated reply badge: {}",
        stdout
    );
    assert!(
        stdout.contains("[FYI]"),
        "history shows default fyi: {}",
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
    assert!(
        output.status.success(),
        "listen that received messages should exit 0: {}",
        stdout
    );
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

// --- Cycle-12: sender read receipts (B021) ---
// Daemon-side per-(session, sender) read state, exposed as `read_by` in
// list/list --json and as a `read: <agent>@<id>` marker in list text.

/// Create a project dir with an identity config and return its path.
fn tala_identity_dir(home: &std::path::Path, name: &str) -> std::path::PathBuf {
    let dir = home.join(format!("ident-{}", name));
    std::fs::create_dir_all(dir.join(".tala")).unwrap();
    std::fs::write(
        dir.join(".tala").join("config.json"),
        format!("{{\"name\": \"{}\"}}", name),
    )
    .unwrap();
    dir
}

#[test]
fn test_read_receipts_after_history() {
    let home = tempfile::tempdir().unwrap();
    let alpha = tala_identity_dir(home.path(), "alpha");
    let beta = tala_identity_dir(home.path(), "beta");

    // alpha creates a session and sends the question.
    let sess = tala_start(home.path());
    tala_ok(
        home.path(),
        &["send", "--session", &sess, "question for beta"],
    );

    // beta reads it via history (identity from beta dir config).
    tala_in(home.path(), Some(&beta), &["history", &sess]);

    // alpha's list --json must show read_by: {"beta": 1}.
    let list = tala_in(home.path(), Some(&alpha), &["list", "--json"]).0;
    let parsed: serde_json::Value = serde_json::from_str(list.trim()).unwrap();
    let session = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["session_id"] == serde_json::json!(sess))
        .expect("session in list");
    assert_eq!(
        session["read_by"],
        serde_json::json!({"beta": 1}),
        "alpha should see beta read msg 1: {}",
        session
    );

    // alpha's text list must show the read marker for beta.
    let text = tala_in(home.path(), Some(&alpha), &["list"]).0;
    assert!(
        text.contains("read: beta@1"),
        "text list should show read: beta@1, got:\n{}",
        text
    );

    tala_stop(home.path());
}

#[test]
fn test_read_receipts_after_wait() {
    let home = tempfile::tempdir().unwrap();
    let alpha = tala_identity_dir(home.path(), "alpha");
    let beta = tala_identity_dir(home.path(), "beta");

    let sess = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &sess, "question"]);

    // beta wait receives the message (sender=beta via identity config).
    let wait = tala_in(
        home.path(),
        Some(&beta),
        &[
            "wait",
            "--session",
            &sess,
            "--since",
            "0",
            "--timeout",
            "10",
        ],
    )
    .0;
    assert!(wait.contains("question"), "wait should deliver: {}", wait);

    let list = tala_in(home.path(), Some(&alpha), &["list", "--json"]).0;
    let parsed: serde_json::Value = serde_json::from_str(list.trim()).unwrap();
    let session = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["session_id"] == serde_json::json!(sess))
        .unwrap();
    assert_eq!(
        session["read_by"],
        serde_json::json!({"beta": 1}),
        "wait should record beta@1: {}",
        session
    );

    tala_stop(home.path());
}

#[test]
fn test_read_receipts_not_recorded_on_send() {
    let home = tempfile::tempdir().unwrap();
    let alpha = tala_identity_dir(home.path(), "alpha");
    let beta = tala_identity_dir(home.path(), "beta");

    let sess = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &sess, "question"]);
    tala_in(home.path(), Some(&beta), &["history", &sess]); // beta reads msg 1

    // alpha sends a follow-up: sending must NOT advance anyone's read state.
    tala_ok(home.path(), &["send", "--session", &sess, "follow-up"]);

    let list = tala_in(home.path(), Some(&alpha), &["list", "--json"]).0;
    let parsed: serde_json::Value = serde_json::from_str(list.trim()).unwrap();
    let session = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["session_id"] == serde_json::json!(sess))
        .unwrap();
    assert_eq!(
        session["read_by"],
        serde_json::json!({"beta": 1}),
        "send must not record read state: {}",
        session
    );

    tala_stop(home.path());
}

#[test]
fn test_read_receipts_self_read_json_but_not_text() {
    let home = tempfile::tempdir().unwrap();
    let alpha = tala_identity_dir(home.path(), "alpha");
    let beta = tala_identity_dir(home.path(), "beta");

    let sess = tala_start(home.path());
    tala_ok(home.path(), &["send", "--session", &sess, "question"]);
    tala_in(home.path(), Some(&beta), &["history", &sess]); // beta reads msg 1

    // alpha re-reads her own session.
    let recap = tala_in(home.path(), Some(&alpha), &["history", &sess]).0;
    assert!(
        recap.contains("question"),
        "alpha history should work: {}",
        recap
    );

    // JSON keeps the full map (self included).
    let list = tala_in(home.path(), Some(&alpha), &["list", "--json"]).0;
    let parsed: serde_json::Value = serde_json::from_str(list.trim()).unwrap();
    let session = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["session_id"] == serde_json::json!(sess))
        .unwrap();
    assert_eq!(
        session["read_by"],
        serde_json::json!({"alpha": 1, "beta": 1}),
        "json read_by should include self: {}",
        session
    );

    // Text hides self-reads.
    let text = tala_in(home.path(), Some(&alpha), &["list"]).0;
    assert!(
        text.contains("read: beta@1"),
        "text should show beta reader: {}",
        text
    );
    assert!(
        !text.contains("read: alpha@"),
        "text must not show self-reads: {}",
        text
    );

    tala_stop(home.path());
}
#[test]
fn test_send_by_name_routes_to_named_session() {
    let home = tempfile::tempdir().unwrap();
    let project_b = tempfile::tempdir().unwrap();

    // Session A (via tala_start's own project dir), renamed to "target-name"
    let sess_a = tala_start(home.path());
    rename_session_by_id(home.path(), &sess_a, "target-name");

    // Session B in project_b, active there
    let sess_b = tala_start(home.path());
    rename_session_by_id(home.path(), &sess_b, "other-name");

    // From project_b (active=sess_b), send to the NAME target-name
    let out = tala_in(
        home.path(),
        Some(project_b.path()),
        &["send", "target-name", "hello-by-name"],
    );
    assert!(out.2, "send by name should succeed: {}", out.0);

    // Message must land in sess_a (named), NOT in active sess_b
    let recap_a = tala_ok(home.path(), &["history", &sess_a]);
    assert!(
        recap_a.contains("hello-by-name"),
        "named session should receive the message: {}",
        recap_a
    );
    let recap_b = tala_ok(home.path(), &["history", &sess_b]);
    assert!(
        !recap_b.contains("hello-by-name"),
        "active session must NOT receive a message addressed by name: {}",
        recap_b
    );

    tala_stop(home.path());
}

#[test]
fn test_send_invalid_intent_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "--intent", "maybe", "hi"],
    );
    assert!(!ok, "invalid intent should fail");
    assert!(
        stderr.contains("invalid --intent"),
        "error should name the flag: {}",
        stderr
    );

    let (_stdout, stderr, ok) = tala(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "out",
            "--expect-reply",
            "bye",
        ],
    );
    assert!(!ok, "expect-reply with out should fail");
    assert!(
        stderr.contains("--expect-reply is only valid"),
        "error should explain modifier rule: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_send_session_flag_accepts_name() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "flag-name");

    let out = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--session", "flag-name", "hello-via-flag"],
    );
    assert!(out.2, "send --session <name> should succeed: {}", out.0);

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("hello-via-flag"),
        "message should land in the named session: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_send_by_name_unknown_errors_without_sending() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "no-such-session-name", "should-not-send"],
    );
    assert!(!ok, "send to unknown name should fail");
    assert!(
        stderr.contains("no-such-session-name") || stdout.contains("no-such-session-name"),
        "error should mention the unknown name: stderr={} stdout={}",
        stderr,
        stdout
    );

    // Nothing may be sent to the active session
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap.contains("should-not-send"),
        "message must not be silently sent to another session: {}",
        recap
    );

    tala_stop(home.path());
}

#[test]
fn test_send_reply_to_invalid_id_rejected() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, stderr, ok) = tala(
        home.path(),
        &["send", "--session", &sess, "--reply-to", "999", "hi"],
    );
    assert!(!ok, "reply to nonexistent id should fail");
    assert!(stderr.contains("999"), "error names the bad id: {}", stderr);

    tala_stop(home.path());
}

#[test]
fn test_history_by_name() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "hist-name");

    tala_ok(home.path(), &["send", &sess, "message for history-by-name"]);

    let out = tala_ok(home.path(), &["history", "hist-name"]);
    assert!(
        out.contains("message for history-by-name"),
        "history by name should show the message: {}",
        out
    );

    tala_stop(home.path());
}

#[test]
fn test_pending_lists_and_clears() {
    let home = tempfile::tempdir().unwrap();
    let (sess, project_a) = tala_start_in(home.path(), "proj-a");
    let (_other_sess, project_b) = tala_start_in(home.path(), "proj-b");

    tala_in(
        home.path(),
        Some(&project_a),
        &["send", "--session", &sess, "--intent", "req", "help me"],
    );
    let (stdout, _stderr, ok) = tala_in(home.path(), Some(&project_a), &["pending"]);
    assert!(ok, "pending should succeed");
    assert!(
        stdout.contains("help me"),
        "pending lists the request: {}",
        stdout
    );
    assert!(
        stdout.contains("awaiting reply"),
        "pending says who owes whom (own req is owed to me): {}",
        stdout
    );

    tala_in(
        home.path(),
        Some(&project_b),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "reply",
            "--reply-to",
            "1",
            "fixed",
        ],
    );
    let (stdout, _stderr, _) = tala_in(home.path(), Some(&project_a), &["pending"]);
    assert!(
        !stdout.contains("help me"),
        "answered request leaves pending: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_close_by_name() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "close-name");

    let out = tala_in(home.path(), None, &["close", "close-name"]);
    assert!(out.2, "close by name should succeed: {}", out.0);

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("close-name") && list.contains("closed"),
        "list should show the session closed: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_rename_by_name() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "old-name");

    let out = tala_in(
        home.path(),
        None,
        &["session", "rename", "old-name", "new-name"],
    );
    assert!(out.2, "rename by name should succeed: {}", out.0);

    let list = tala_ok(home.path(), &["list"]);
    assert!(
        list.contains("new-name") && !list.contains("old-name"),
        "list should show the new name: {}",
        list
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_by_name() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "wait-name");

    // Wait with no traffic should time out cleanly (and resolve the name first)
    let (stdout, stderr, ok) = tala(home.path(), &["wait", "wait-name", "--timeout", "2"]);
    assert!(
        !stderr.contains("not found"),
        "wait by name must resolve the name, not fail: stderr={}",
        stderr
    );
    assert!(
        ok || stderr.contains("timeout"),
        "wait by name on empty session should time out: ok={} stdout={} stderr={}",
        ok,
        stdout,
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_lone_positional_is_message_not_session() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let sess = tala_start(home.path());
    rename_session_by_id(home.path(), &sess, "lone-name");

    // A lone positional that is NOT a session name must remain a message to active
    let out = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "just a plain message"],
    );
    assert!(out.2, "lone positional should send as message: {}", out.0);

    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        recap.contains("just a plain message"),
        "lone positional should go to active session: {}",
        recap
    );

    tala_stop(home.path());
}
fn rename_session_by_id(home: &std::path::Path, sess: &str, name: &str) {
    tala_ok(home, &["session", "rename", sess, name, "--force"]);
}
#[test]
fn test_status_shows_home_text() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let status = tala_ok(home.path(), &["status"]);
    assert!(
        status.contains("Home:"),
        "status text should show Home line: {}",
        status
    );

    tala_stop(home.path());
}

#[test]
fn test_pending_excludes_closed_sessions() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "req",
            "orphan-request",
        ],
    );
    tala_ok(home.path(), &["close", &sess]);

    let (stdout, _stderr, _) = tala(home.path(), &["pending"]);
    assert!(
        !stdout.contains("orphan-request"),
        "closed sessions excluded from pending: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_send_wait_strict_reply_matching() {
    let home = tempfile::tempdir().unwrap();
    let (sess, project) = tala_start_in(home.path(), "proj-a");
    let (_other_sess, other_project) = tala_start_in(home.path(), "proj-b");

    tala_in(
        home.path(),
        Some(&project),
        &["send", "--session", &sess, "--intent", "req", "question?"],
    );

    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .current_dir(&project)
        .args([
            "send",
            "--session",
            &sess,
            "--wait",
            "--timeout",
            "15",
            "question2?",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1500));

    // Unrelated FYI from another agent must NOT satisfy the strict wait
    tala_in(
        home.path(),
        Some(&other_project),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "fyi",
            "unrelated-chatter",
        ],
    );
    std::thread::sleep(std::time::Duration::from_millis(1500));
    assert!(
        child.try_wait().unwrap().is_none(),
        "unrelated fyi should not end the strict wait"
    );

    // The correlated reply ends the wait
    tala_in(
        home.path(),
        Some(&other_project),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "reply",
            "--reply-to",
            "2",
            "the-real-answer",
        ],
    );
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "wait should succeed: {}", stdout);
    assert!(
        stdout.contains("the-real-answer"),
        "wait returns the correlated reply: {}",
        stdout
    );
    assert!(
        !stdout.contains("unrelated-chatter"),
        "wait must not return the unrelated fyi: {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_status_json_has_home_and_flag() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    // Without TALA_HOME: tala_home_set must be false, home must be present.
    let (stdout, _stderr, ok) = tala(home.path(), &["status", "--json"]);
    assert!(ok, "status --json should succeed");
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(v["running"], true);
    assert_eq!(
        v["tala_home_set"], false,
        "no TALA_HOME in env for this test"
    );
    let home_str = v["home"].as_str().expect("home field present");
    assert!(
        home_str.contains(".tala"),
        "home should point into .tala: {}",
        home_str
    );

    // With TALA_HOME set: flag must be true and home must match it.
    let custom = home.path().join("custom-home");
    std::fs::create_dir_all(&custom).unwrap();
    let (stdout, _stderr, ok) = tala_in_env(home.path(), Some(&custom), &["status", "--json"]);
    assert!(ok, "status --json (TALA_HOME set) should succeed");
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(v["tala_home_set"], true, "TALA_HOME was set for this test");
    assert_eq!(
        v["home"].as_str().unwrap(),
        custom.to_str().unwrap(),
        "home should be the TALA_HOME path"
    );

    tala_stop(home.path());
}

#[test]
fn test_status_no_daemon_json_home() {
    let home = tempfile::tempdir().unwrap();

    let (stdout, _stderr, ok) = tala(home.path(), &["status", "--json"]);
    assert!(ok, "status --json should succeed even with no daemon");
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(v["running"], false);
    assert!(
        v["home"].as_str().is_some(),
        "not-running status --json should include home: {}",
        stdout
    );
}

#[test]
fn test_status_warns_on_default_home() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    // TALA_HOME unset (default), daemon running -> stderr warning.
    let (stdout, stderr, ok) = tala(home.path(), &["status"]);
    assert!(ok, "status should succeed: {}", stdout);
    assert!(
        stderr.contains("TALA_HOME is not set") && stderr.contains("default daemon home"),
        "stderr should warn about default home: {}",
        stderr
    );

    tala_stop(home.path());
}
fn tala_in_env(
    home: &std::path::Path,
    tala_home: Option<&std::path::Path>,
    args: &[&str],
) -> (String, String, bool) {
    let mut cmd = Command::new(tala_bin());
    cmd.env("HOME", home);
    if let Some(th) = tala_home {
        cmd.env("TALA_HOME", th);
    } else {
        cmd.env_remove("TALA_HOME");
    }
    cmd.args(args);
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run tala {}: {}", args.join(" "), e));
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}
#[test]
fn test_send_sender_mismatch_rejected() {
    // B004 (PO decision 2026-08-07): --sender is RESTRICTED to the project's
    // configured agent name. A mismatched identity is a hard error: nothing is
    // sent, exit non-zero, stderr names both identities.
    let home = tempfile::tempdir().unwrap();
    let project = init_project(home.path(), "agent-alpha");
    let sess = create_session_in(home.path(), project.path());

    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "spoofed-agent",
            "impersonation probe",
        ],
    );
    assert!(
        !ok,
        "send with mismatched --sender must FAIL\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("spoofed-agent") && stderr.contains("agent-alpha"),
        "error should name both identities: {stderr}"
    );
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap.contains("impersonation probe"),
        "mismatched --sender must not send the message: {recap}"
    );

    // Matching --sender: still succeeds, no error.
    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "agent-alpha",
            "legit message",
        ],
    );
    assert!(ok, "send with matching --sender should succeed: {stdout}");
    assert!(
        !stderr.contains("Warning: sending as") && !stderr.contains("Error:"),
        "matching --sender must not error: {stderr}"
    );

    tala_stop(home.path());
}

#[test]
fn test_wait_overlap_warning() {
    let home = tempfile::tempdir().unwrap();
    let (sess, project) = tala_start_in(home.path(), "proj-a");

    let mut first = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .current_dir(&project)
        .args(["wait", "--session", &sess, "--timeout", "20"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1500));

    let (_stdout, stderr, ok) = tala(home.path(), &["wait", "--session", &sess, "--timeout", "2"]);
    assert!(!ok, "second wait times out with exit 2");
    assert!(
        stderr.contains("is waiting on"),
        "overlap note expected: {}",
        stderr
    );

    let _ = first.kill();
    let _ = first.wait();
    tala_stop(home.path());
}

#[test]
fn test_send_sender_mismatch_json_error() {
    // B004 --json contract: mismatch emits a machine-readable
    // {"error": ..., "code": "SENDER_MISMATCH"} on stderr, exit 1, nothing sent.
    let home = tempfile::tempdir().unwrap();
    let project = init_project(home.path(), "agent-alpha");
    let sess = create_session_in(home.path(), project.path());

    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "spoofed-agent",
            "json probe",
            "--json",
        ],
    );
    assert!(
        !ok,
        "send --json with mismatched --sender must FAIL: {stdout}"
    );
    let err: serde_json::Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|_| panic!("--json mismatch must emit JSON error: {stderr}"));
    assert_eq!(
        err["code"],
        serde_json::Value::String("SENDER_MISMATCH".into()),
        "error code must be SENDER_MISMATCH: {stderr}"
    );
    assert!(
        err["error"].as_str().unwrap_or("").contains("agent-alpha"),
        "error must name the configured agent: {stderr}"
    );
    let recap = tala_ok(home.path(), &["history", &sess]);
    assert!(
        !recap.contains("json probe"),
        "mismatched --sender must not send in --json mode: {recap}"
    );

    // Matching --sender with --json: succeeds, no mismatch fields.
    let (stdout, _stderr, _ok) = tala_in(
        home.path(),
        Some(project.path()),
        &[
            "send",
            "--session",
            &sess,
            "--sender",
            "agent-alpha",
            "json legit",
            "--json",
        ],
    );
    let val: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("send --json emits JSON");
    assert_ne!(
        val.get("sender_mismatch"),
        Some(&serde_json::Value::Bool(true)),
        "matching sender must not flag a mismatch: {stdout}"
    );

    tala_stop(home.path());
}

fn init_project(home: &std::path::Path, name: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let (stdout, stderr, ok) = tala_in(home, Some(project.path()), &["init", name]);
    assert!(
        ok,
        "tala init {name} failed\nstdout: {stdout}\nstderr: {stderr}"
    );
    project
}
fn create_session_in(home: &std::path::Path, project: &std::path::Path) -> String {
    let (stdout, stderr, ok) = tala_in(home, Some(project), &["session", "create"]);
    assert!(
        ok,
        "tala session create failed\nstdout: {stdout}\nstderr: {stderr}"
    );
    stdout.lines().next().unwrap_or("").trim().to_string()
}
#[test]
fn test_create_duplicate_name_rejected() {
    let home = tempfile::tempdir().unwrap();
    let first = tala_start(home.path());

    tala_ok(
        home.path(),
        &["session", "rename", &first, "dup-a", "--force"],
    );

    // Second create with the same name must fail loudly.
    let (stdout, stderr, ok) = tala(home.path(), &["session", "create", "--name", "dup-a"]);
    assert!(!ok, "duplicate name create should fail, got: {}", stdout);
    assert!(
        stderr.contains("already exists"),
        "stderr should explain the duplicate: {}",
        stderr
    );

    // A different name still succeeds.
    let (stdout2, _stderr2, ok2) = tala(home.path(), &["session", "create", "--name", "dup-a2"]);
    assert!(ok2, "distinct name create should succeed: {}", stdout2);

    tala_stop(home.path());
}

#[test]
fn test_wait_new_session_timeout_hint() {
    let home = tempfile::tempdir().unwrap();
    let (sess, project) = tala_start_in(home.path(), "proj-a");
    let (_other_sess, other_project) = tala_start_in(home.path(), "proj-b");

    tala_in(
        home.path(),
        Some(&other_project),
        &["send", "--session", &sess, "msg-for-you"],
    );

    let (_stdout, stderr, ok) = tala_in(
        home.path(),
        Some(&project),
        &["wait", "--new-session", "--timeout", "2"],
    );
    assert!(!ok, "wait --new-session timeout exits 2");
    assert!(
        stderr.contains("unread message"),
        "timeout hint expected: {}",
        stderr
    );
    assert!(stderr.contains(&sess), "hint names the session: {}", stderr);

    tala_stop(home.path());
}

#[test]
fn test_create_duplicate_name_json_error() {
    let home = tempfile::tempdir().unwrap();
    let first = tala_start(home.path());
    tala_ok(
        home.path(),
        &["session", "rename", &first, "dup-b", "--force"],
    );

    let (stdout, stderr, ok) = tala(
        home.path(),
        &["session", "create", "--name", "dup-b", "--json"],
    );
    assert!(!ok, "duplicate name create --json should fail: {}", stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be JSON");
    assert_eq!(
        parsed["code"],
        serde_json::json!("SESSION_NAME_TAKEN"),
        "json error code: {}",
        stderr
    );
    assert!(
        parsed["error"]
            .as_str()
            .unwrap_or("")
            .contains("already exists"),
        "json error message: {}",
        stderr
    );

    tala_stop(home.path());
}

#[test]
fn test_send_wait_stamps_deadline_and_expires() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let (_stdout, _stderr, ok) = tala(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "req",
            "--wait",
            "--timeout",
            "1",
            "urgent-question",
        ],
    );
    assert!(!ok, "wait with no reply should time out (exit 2)");

    std::thread::sleep(std::time::Duration::from_secs(2));
    let (stdout, _stderr, _) = tala(home.path(), &["history", "--session", &sess]);
    assert!(
        !stdout.contains("wait expired"),
        "history must not render expired deadlines (live surfaces only): {}",
        stdout
    );

    tala_stop(home.path());
}

#[test]
fn test_rename_to_existing_name_rejected() {
    let home = tempfile::tempdir().unwrap();
    let s1 = tala_start(home.path());
    let s2 = tala_start(home.path());
    tala_ok(home.path(), &["session", "rename", &s1, "n1", "--force"]);
    tala_ok(home.path(), &["session", "rename", &s2, "n2", "--force"]);

    // Renaming s1 onto s2's name must fail.
    let (stdout, stderr, ok) = tala(home.path(), &["session", "rename", &s1, "n2"]);
    assert!(!ok, "rename onto existing name should fail: {}", stdout);
    assert!(
        stderr.contains("already exists"),
        "stderr should explain the duplicate: {}",
        stderr
    );

    // Renaming s2 onto s1's name must fail too (symmetric).
    let (stdout2, _stderr2, ok2) = tala(home.path(), &["session", "rename", &s2, "n1"]);
    assert!(!ok2, "rename onto existing name should fail: {}", stdout2);

    // Renaming s1 to its OWN current name stays a success (noop).
    let (stdout3, _stderr3, ok3) = tala(home.path(), &["session", "rename", &s1, "n1"]);
    assert!(ok3, "rename to own name should succeed: {}", stdout3);

    tala_stop(home.path());
}

#[test]
fn test_rename_duplicate_json_error() {
    let home = tempfile::tempdir().unwrap();
    let s1 = tala_start(home.path());
    let s2 = tala_start(home.path());
    tala_ok(home.path(), &["session", "rename", &s1, "dup-c", "--force"]);
    tala_ok(home.path(), &["session", "rename", &s2, "dup-d", "--force"]);

    let (stdout, stderr, ok) = tala(home.path(), &["session", "rename", &s1, "dup-d", "--json"]);
    assert!(!ok, "duplicate rename --json should fail: {}", stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stderr.trim()).expect("stderr should be JSON");
    assert_eq!(
        parsed["code"],
        serde_json::json!("SESSION_NAME_TAKEN"),
        "json error code: {}",
        stderr
    );
    assert!(
        parsed["error"]
            .as_str()
            .unwrap_or("")
            .contains("already exists"),
        "json error message: {}",
        stderr
    );

    tala_stop(home.path());
}
#[test]
fn test_broken_pipe_no_panic() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    // Seed enough messages that history --json output is large (B038 repro
    // needs the writer to still be writing when the reader closes).
    for i in 0..400 {
        tala_ok(
            home.path(),
            &["send", "--session", &sess, &format!("bulk-{}", i)],
        );
    }

    use std::process::Stdio;
    let mut child = Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["history", &sess, "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tala history");

    // Close the read end immediately: every subsequent write gets EPIPE.
    drop(child.stdout.take());

    let out = child.wait_with_output().expect("wait for tala history");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "broken pipe must not panic:\n{}",
        stderr
    );
    assert_ne!(
        out.status.code(),
        Some(101),
        "broken pipe must not exit with the panic code"
    );

    tala_stop(home.path());
}

// ---- adopt-a2a-principles: message parts ----

#[test]
fn test_parts_send_and_render() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--part",
            "text:review the change",
            "--part",
            "file:src/api.rs",
            "--part",
            r#"data:{"status":"ok"}"#,
        ],
    );

    let hist = tala_ok(home.path(), &["history", &sess]);
    assert!(
        hist.contains("review the change"),
        "text part should render as content: {}",
        hist
    );
    assert!(
        hist.contains("[file: src/api.rs]"),
        "file part should render as annotation: {}",
        hist
    );
    assert!(
        hist.contains(r#"[data: {"status":"ok"}]"#),
        "data part should render as annotation: {}",
        hist
    );

    // --json exposes the typed parts array.
    let json_out = tala_ok(home.path(), &["history", &sess, "--json"]);
    let val: serde_json::Value = serde_json::from_str(&json_out).unwrap();
    let msg = &val["messages"][0];
    let parts = msg["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[1]["type"], "file");
    assert_eq!(parts[1]["path"], "src/api.rs");
    assert_eq!(parts[2]["type"], "data");
    // Legacy content view still present for older clients.
    assert_eq!(msg["content"], "review the change");

    tala_stop(home.path());
}

#[test]
fn test_parts_only_send_creates_session() {
    let home = tempfile::tempdir().unwrap();
    let project = init_project(home.path(), "parts-agent");

    let (stdout, stderr, ok) = tala_in(
        home.path(),
        Some(project.path()),
        &["send", "--part", "text:hello", "--part", "file:x"],
    );
    assert!(
        ok,
        "parts-only send should auto-create a session: {stdout} {stderr}"
    );
    assert!(
        stdout.contains("✓ Sent message 1"),
        "parts-only send should succeed: {}",
        stdout
    );
    tala_stop(home.path());
}

#[test]
fn test_parts_validation_errors() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    let cases: &[(&[&str], &str)] = &[
        (
            &["send", "--session", &sess, "--part", "bogus:value"],
            "invalid --part kind",
        ),
        (
            &["send", "--session", &sess, "--part", "text:"],
            "empty text part",
        ),
        (
            &["send", "--session", &sess, "--part", "data:not-json"],
            "invalid --part data",
        ),
        (
            &["send", "--session", &sess, "pos", "--part", "text:x"],
            "--part cannot be combined",
        ),
    ];
    for (args, needle) in cases {
        let (stdout, stderr, ok) = tala(home.path(), args);
        assert!(!ok, "{args:?} should fail, got: {stdout}");
        assert!(
            stderr.contains(needle) || stdout.contains(needle),
            "{args:?} stderr should mention '{needle}', got: {stderr}"
        );
    }

    tala_stop(home.path());
}

#[test]
fn test_legacy_messages_load_after_restart() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    tala_ok(home.path(), &["send", "--session", &sess, "legacy text"]);

    // Restart the daemon: persisted legacy messages must load as text parts.
    tala_stop(home.path());
    let sess2 = tala_start(home.path());
    let hist = tala_ok(home.path(), &["history", &sess]);
    assert!(
        hist.contains("legacy text"),
        "legacy message must survive restart: {} (session {} persisted, restarted daemon on {})",
        hist,
        sess,
        sess2
    );
    tala_stop(home.path());
}

// ---- adopt-a2a-principles: send idempotency ----

fn daemon_addr(home: &std::path::Path) -> (String, u16) {
    let info: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(home.join(".tala").join("daemon.json")).unwrap(),
    )
    .unwrap();
    (
        info["host"].as_str().unwrap_or("127.0.0.1").to_string(),
        info["port"].as_u64().unwrap_or(0) as u16,
    )
}

/// Raw HTTP/1.1 POST against the daemon (the CLI always generates fresh keys,
/// so daemon-side dedup is exercised at the wire level).
fn raw_post(home: &std::path::Path, path: &str, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let (host, port) = daemon_addr(home);
    let mut stream = std::net::TcpStream::connect((host.as_str(), port)).unwrap();
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        host,
        port,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    let status: u16 = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn send_body(sender: &str, content: &str, key: Option<&str>) -> String {
    let mut body = format!(
        r#"{{"sender":"{}","content":"{}""#,
        sender,
        content.replace('"', "\\\"")
    );
    if let Some(k) = key {
        body.push_str(&format!(r#","idempotency_key":"{}""#, k));
    }
    body.push('}');
    body
}

#[test]
fn test_idempotency_dedup_and_conflict() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let path = format!("/api/sessions/{}/messages", sess);

    let (status, body) = raw_post(
        home.path(),
        &path,
        &send_body("wire-agent", "hello", Some("k1")),
    );
    assert_eq!(status, 201, "first send should store: {}", body);
    let first: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(first["duplicate"], false);

    // Retry with the same key and content: deduplicated, original returned.
    let (status, body) = raw_post(
        home.path(),
        &path,
        &send_body("wire-agent", "hello", Some("k1")),
    );
    assert_eq!(status, 200, "duplicate should return OK: {}", body);
    let dup: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(dup["duplicate"], true, "duplicate flag set: {}", body);
    assert_eq!(dup["id"], first["id"], "original message id returned");
    assert_eq!(dup["session_id"], first["session_id"]);

    // Same key, different content: conflict, nothing stored.
    let (status, body) = raw_post(
        home.path(),
        &path,
        &send_body("wire-agent", "different", Some("k1")),
    );
    assert_eq!(status, 409, "key conflict should fail: {}", body);
    assert!(body.contains("conflict"), "conflict named: {}", body);

    // Missing key: rejected.
    let (status, body) = raw_post(home.path(), &path, &send_body("wire-agent", "no-key", None));
    assert_eq!(status, 400, "missing key rejected: {}", body);

    // Exactly one message stored.
    let hist = tala_ok(home.path(), &["history", &sess, "--json"]);
    let val: serde_json::Value = serde_json::from_str(&hist).unwrap();
    assert_eq!(val["messages"].as_array().unwrap().len(), 1);

    tala_stop(home.path());
}

#[test]
fn test_idempotency_survives_daemon_restart() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let path = format!("/api/sessions/{}/messages", sess);

    let (status, body) = raw_post(
        home.path(),
        &path,
        &send_body("wire-agent", "once", Some("k9")),
    );
    assert_eq!(status, 201, "first send should store: {}", body);

    // Restart the daemon; the dedup index must be rebuilt from persistence.
    tala_stop(home.path());
    tala_start(home.path());
    let (status, body) = raw_post(
        home.path(),
        &path,
        &send_body("wire-agent", "once", Some("k9")),
    );
    assert_eq!(status, 200, "dedup must survive restart: {}", body);
    let dup: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(dup["duplicate"], true);

    let hist = tala_ok(home.path(), &["history", &sess, "--json"]);
    let val: serde_json::Value = serde_json::from_str(&hist).unwrap();
    assert_eq!(
        val["messages"].as_array().unwrap().len(),
        1,
        "retry after restart must not duplicate"
    );

    tala_stop(home.path());
}

// ---- adopt-a2a-principles: daemon version negotiation ----

/// Spawns a fake "stale" daemon: an HTTP server reporting protocol_version 0
/// on /api/status, plus empty list endpoints. Writes daemon.json in `home`.
fn fake_stale_daemon(home: &std::path::Path) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for _ in 0..32 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            let path = req.split_whitespace().nth(1).unwrap_or("/");
            let body = match path {
                p if p.starts_with("/api/status") => {
                    r#"{"pid":1,"port":1,"uptime_seconds":1,"session_count":0,"protocol_version":0}"#.to_string()
                }
                p if p.starts_with("/api/sessions") => "[]".to_string(),
                p if p.starts_with("/api/waits") => "[]".to_string(),
                p if p.starts_with("/api/agents") => "[]".to_string(),
                _ => r#"{"error":"not found"}"#.to_string(),
            };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let info = serde_json::json!({
        "pid": 1,
        "port": port,
        "host": "127.0.0.1",
        "started_at": "2024-01-01T00:00:00Z",
        "protocol_version": 0,
    });
    let daemon_dir = home.join(".tala");
    std::fs::create_dir_all(&daemon_dir).unwrap();
    std::fs::write(
        daemon_dir.join("daemon.json"),
        serde_json::to_string(&info).unwrap(),
    )
    .unwrap();
}

#[test]
fn test_stale_daemon_blocks_commands() {
    let home = tempfile::tempdir().unwrap();
    fake_stale_daemon(home.path());

    let (stdout, stderr, ok) = tala(home.path(), &["send", "--json", "hello"]);
    assert!(!ok, "send must fail against a stale daemon: {}", stdout);
    let err: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(
        err["code"], "VERSION_MISMATCH",
        "json error doc: {}",
        stderr
    );
    assert!(
        err["error"]
            .as_str()
            .unwrap_or("")
            .contains("protocol version 0"),
        "error names the versions: {}",
        stderr
    );

    let (stdout, stderr, ok) = tala(home.path(), &["history", "sess_x", "--json"]);
    assert!(!ok, "history must fail against a stale daemon: {}", stdout);
    assert!(stderr.contains("VERSION_MISMATCH"), "stderr: {}", stderr);
}

#[test]
fn test_stale_daemon_read_only_commands_warn() {
    let home = tempfile::tempdir().unwrap();
    fake_stale_daemon(home.path());

    // status: warns, still exits 0, still reports the version.
    let (stdout, stderr, ok) = tala(home.path(), &["status", "--json"]);
    assert!(ok, "status must not fail: {}", stdout);
    assert!(
        stderr.contains("incompatible"),
        "status should warn about the mismatch: {}",
        stderr
    );
    let val: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(val["protocol_version"], 0);

    // discover: warn, exit 0.
    let (stdout, stderr, ok) = tala(home.path(), &["discover", "--json"]);
    assert!(ok, "discover must not fail: {}", stdout);
    assert!(
        stderr.contains("incompatible"),
        "discover should warn: {}",
        stderr
    );
}

#[test]
fn test_status_reports_protocol_version() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    let human = tala_ok(home.path(), &["status"]);
    assert!(
        human.contains("Protocol: 1"),
        "status should show the protocol version: {}",
        human
    );

    let json_out = tala_ok(home.path(), &["status", "--json"]);
    let val: serde_json::Value = serde_json::from_str(&json_out).unwrap();
    assert_eq!(val["protocol_version"], 1);

    tala_stop(home.path());
}

// --- Cycle-18 eval feedback fixes (B039-B041) ---

#[test]
fn test_session_create_json_outputs_session_id() {
    let home = tempfile::tempdir().unwrap();
    let (stdout, stderr, ok) = tala(
        home.path(),
        &["session", "create", "--name", "json-probe", "--json"],
    );
    assert!(
        ok,
        "session create --json should succeed: {} {}",
        stdout, stderr
    );
    let val: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--json success must emit a JSON doc");
    let sid = val["session_id"]
        .as_str()
        .expect("JSON must contain session_id");
    assert!(sid.starts_with("sess_"), "session_id shape: {}", sid);
    tala_stop(home.path());
}

#[test]
fn test_send_reply_intent_without_reply_to_warns() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let (stdout, stderr, ok) = tala(
        home.path(),
        &[
            "send",
            "--session",
            &sess,
            "--intent",
            "reply",
            "uncorrelated answer",
        ],
    );
    assert!(ok, "send should still succeed: {} {}", stdout, stderr);
    assert!(
        stderr.contains("--reply-to"),
        "reply intent without --reply-to should warn on stderr: {}",
        stderr
    );
    assert!(
        !stdout.contains("--reply-to"),
        "warning must not pollute stdout: {}",
        stdout
    );
    tala_stop(home.path());
}

#[test]
fn test_wait_new_json_includes_session_name() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["session", "create", "--name", "named-handshake"],
    );
    assert!(ok, "alpha session create failed: {}", sout);
    let alpha_sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &alpha_sess, "hello from alpha"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // --json carries the name
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10", "--json"],
    );
    assert!(ok, "wait --new-session --json should succeed: {}", stdout);
    assert!(
        stdout.contains("named-handshake"),
        "wait-new --json should include the session name: {}",
        stdout
    );

    // text mode keeps the bare id on stdout (capture contract), name on stderr
    let (sout2, _serr2, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["session", "create", "--name", "named-handshake-2"],
    );
    assert!(ok, "alpha second session create failed: {}", sout2);
    let sess2 = sout2.trim().to_string();
    let (sout3, _serr3, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "--session", &sess2, "second hello"],
    );
    assert!(ok, "alpha second send failed: {}", sout3);
    let (stdout_text, stderr_text, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["wait", "--new-session", "--timeout", "10"],
    );
    assert!(
        ok,
        "text wait --new-session should succeed: {}",
        stdout_text
    );
    assert_eq!(
        stdout_text.trim(),
        sess2,
        "text wait-new must print the bare session id on stdout"
    );
    assert!(
        stderr_text.contains("named-handshake-2"),
        "text wait-new should print the session name as stderr context: {}",
        stderr_text
    );

    tala_stop(home.path());
}

// --- Cycle-19 surface narrowing tests ---

#[test]
fn test_send_name_creates_named_session() {
    let home = tempfile::tempdir().unwrap();
    let (stdout, stderr, ok) = tala(
        home.path(),
        &["send", "--name", "named-start", "first message"],
    );
    assert!(ok, "send --name should succeed: {} {}", stdout, stderr);
    assert!(
        stdout.contains("sess_"),
        "send --name should print the session id: {}",
        stdout
    );
    let listed = tala_ok(home.path(), &["list"]);
    assert!(
        listed.contains("named-start"),
        "list should show the named session: {}",
        listed
    );
    let hist = tala_ok(home.path(), &["history"]);
    assert!(
        hist.contains("first message"),
        "history should show the message: {}",
        hist
    );
    tala_stop(home.path());
}

#[test]
fn test_send_name_with_explicit_session_errors() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let (stdout, stderr, ok) = tala(
        home.path(),
        &["send", "-s", &sess, "--name", "conflict", "message"],
    );
    assert!(!ok, "send --name with explicit session must fail");
    assert!(
        stdout.contains("--name") || stderr.contains("--name"),
        "error should mention --name: {} {}",
        stdout,
        stderr
    );
    tala_stop(home.path());
}

#[test]
fn test_wait_timeout_zero_is_indefinite() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    // A wait with --timeout 0 must stay parked past the old "instant timeout"
    // behavior and deliver a message that arrives after it starts.
    let child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["wait", "-s", &sess, "--timeout", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start wait");

    std::thread::sleep(std::time::Duration::from_millis(1500));
    tala_ok(
        home.path(),
        &["send", "-s", &sess, "delivered-while-parked"],
    );

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("delivered-while-parked"),
        "timeout-0 wait should deliver the message: {}",
        stdout
    );
    assert!(
        output.status.success(),
        "timeout-0 wait should exit 0 on delivery"
    );
    tala_stop(home.path());
}

#[test]
fn test_bare_send_warns_with_multiple_open_sessions() {
    let home = tempfile::tempdir().unwrap();
    // Two sessions created from two project dirs; p-a holds the active marker.
    let (a, project_a) = tala_start_in(home.path(), "p-a");
    let (_b, _project_b) = tala_start_in(home.path(), "p-b");

    let (stdout, stderr, ok) =
        tala_in(home.path(), Some(&project_a), &["send", "bare-target-test"]);
    assert!(ok, "bare send should still succeed: {} {}", stdout, stderr);
    assert!(
        stderr.contains("open sessions"),
        "bare send with 2 open sessions should warn on stderr: {}",
        stderr
    );
    assert!(
        stdout.contains("Sent message"),
        "message should be sent: {}",
        stdout
    );
    // Message landed in the active session (p-a's session).
    let listed = tala_ok(home.path(), &["list"]);
    let a_line = listed.lines().find(|l| l.contains(&a)).unwrap_or("");
    assert!(
        a_line.contains("1 msgs"),
        "active session should hold the message: {}",
        listed
    );
    tala_stop(home.path());
}

// --- Cycle-20 listen trust tests (B046) ---

#[test]
fn test_listen_timeout_zero_stays_connected_and_delivers() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());

    // --timeout 0 must NOT be converted to 60 (B046): the listener stays
    // connected and delivers a message sent after it connects.
    let mut child = std::process::Command::new(tala_bin())
        .env("HOME", home.path())
        .args(["listen", "--timeout", "0", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start listen");

    std::thread::sleep(std::time::Duration::from_millis(1200));
    tala_ok(home.path(), &["send", "-s", &sess, "listener-gets-this"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Listener must still be alive (no 0s/60s timeout closed it) and must
    // have received the message: delivery while connected proves liveness.
    // Kill first (timeout 0 never exits on its own), then drain the pipe.
    let _ = child.kill();
    let _ = child.wait();
    let mut stdout_buf = String::new();
    use std::io::Read;
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout_buf)
        .ok();
    assert!(
        stdout_buf.contains("listener-gets-this"),
        "listen --timeout 0 should deliver the message: {}",
        stdout_buf
    );
    tala_stop(home.path());
}

#[test]
fn test_listen_advances_cursor_check_agrees() {
    let home = tempfile::tempdir().unwrap();
    // Alpha sends; beta monitors from a different project dir (fresh cursors).
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "alpha session create failed: {}", sout);
    let sess = sout.trim().to_string();
    let (sout, _serr, ok) = tala_in(
        home.path(),
        Some(&alpha_proj),
        &["send", "-s", &sess, "seen-by-listener"],
    );
    assert!(ok, "alpha send failed: {}", sout);

    // Beta listens: the pre-existing message is replayed (beta cursor = 0),
    // and listen advances beta's cursor as it delivers.
    let (stdout, _stderr, ok) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["listen", "--timeout", "15", "--json"],
    );
    assert!(ok, "listen should succeed: {}", stdout);
    assert!(
        stdout.contains("seen-by-listener"),
        "listen should deliver the message: {}",
        stdout
    );

    let (check, _serr2, ok2) = tala_in(home.path(), Some(&beta_proj), &["check", "--json"]);
    assert!(ok2, "check should succeed: {}", check);
    assert!(
        !check.contains("seen-by-listener"),
        "check must not re-show a message listen already delivered: {}",
        check
    );

    // A second listen (replay from beta's cursors) must not replay it either —
    // it times out empty, which is exit 3 (family contract).
    let (stdout2, _stderr3, ok3) = tala_in(
        home.path(),
        Some(&beta_proj),
        &["listen", "--timeout", "2", "--json"],
    );
    assert!(!ok3, "second listen should exit 3: {}", stdout2);
    assert!(
        !stdout2.contains("seen-by-listener"),
        "reconnect must not replay delivered messages: {}",
        stdout2
    );
    tala_stop(home.path());
}

#[test]
fn test_listen_timeout_exits_3() {
    let home = tempfile::tempdir().unwrap();
    tala_start(home.path());

    // Benign timeout exits 3 (family contract with wait/send), not 0.
    let (stdout, stderr, ok) = tala(home.path(), &["listen", "--timeout", "2"]);
    assert!(
        !ok,
        "listen timeout should exit nonzero: {} {}",
        stdout, stderr
    );
    let code = {
        // re-run capturing the exit code
        let out = std::process::Command::new(tala_bin())
            .env("HOME", home.path())
            .args(["listen", "--timeout", "2"])
            .output()
            .unwrap();
        out.status.code().unwrap_or(-1)
    };
    assert_eq!(code, 3, "listen benign timeout should exit 3");
    tala_stop(home.path());
}

#[test]
fn test_wait_stale_active_session_hints_and_clears() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    run_init_in(&project, home.path(), &["init", "alice"]);

    // Fabricate a stale active marker (e.g. left over from another daemon):
    // the session id does not exist on this daemon.
    let tala_dir = project.join(".tala");
    std::fs::create_dir_all(&tala_dir).unwrap();
    std::fs::write(tala_dir.join("active-session"), "sess_deadbeef").unwrap();

    // A bare wait must NOT die with SESSION_NOT_FOUND; it clears the stale
    // marker, hints, and falls back to waiting for a new session (exit 3).
    let (stdout, stderr, ok) = tala_in(home.path(), Some(&project), &["wait", "--timeout", "2"]);
    assert!(
        !ok,
        "bare wait with stale active should exit nonzero: {}",
        stdout
    );
    assert!(
        stderr.contains("cleared"),
        "stale active should be cleared with a hint: {}",
        stderr
    );
    assert!(
        !stdout.contains("SESSION_NOT_FOUND"),
        "stale active must not surface SESSION_NOT_FOUND: {}",
        stdout
    );
    tala_stop(home.path());
}

#[test]
fn test_pending_json_includes_full_content() {
    let home = tempfile::tempdir().unwrap();
    let sess = tala_start(home.path());
    let long = format!(
        "request with a very long body {}",
        "padding-padding-padding-".repeat(30)
    );
    tala_ok(
        home.path(),
        &["send", "-s", &sess, "--intent", "req", &long],
    );

    let (stdout, _stderr, ok) = tala(home.path(), &["pending", "--json"]);
    assert!(ok, "pending --json should succeed: {}", stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let first = &val[0];
    assert!(
        first["content_full"]
            .as_str()
            .unwrap_or("")
            .contains("padding-padding"),
        "pending --json should carry the full content: {}",
        stdout
    );
    tala_stop(home.path());
}

#[test]
fn test_pending_hint_depends_on_who_owes_whom() {
    let home = tempfile::tempdir().unwrap();
    let alpha_proj = home.path().join("alpha-proj");
    let beta_proj = home.path().join("beta-proj");
    std::fs::create_dir_all(&alpha_proj).unwrap();
    std::fs::create_dir_all(&beta_proj).unwrap();
    run_init_in(&alpha_proj, home.path(), &["init", "alpha"]);
    run_init_in(&beta_proj, home.path(), &["init", "beta"]);

    let (sout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["session", "create"]);
    assert!(ok, "session create failed: {}", sout);
    let sess = sout.trim().to_string();

    // Alpha asks (req): alpha's own pending says "awaiting reply", not
    // "answer with --reply-to" (you cannot answer your own question).
    let _ = tala_in(
        home.path(),
        Some(&alpha_proj),
        &[
            "send",
            "-s",
            &sess,
            "--intent",
            "req",
            "question-from-alpha",
        ],
    );
    let (stdout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["pending"]);
    assert!(ok, "pending should succeed: {}", stdout);
    assert!(
        stdout.contains("awaiting reply"),
        "own unanswered req should say 'awaiting reply': {}",
        stdout
    );
    assert!(
        !stdout.contains("answer with"),
        "own unanswered req must not say 'answer with': {}",
        stdout
    );

    // Beta replies with expect-reply: now alpha owes beta — the hint flips.
    let _ = tala_in(
        home.path(),
        Some(&beta_proj),
        &[
            "send",
            "-s",
            &sess,
            "--intent",
            "reply",
            "--reply-to",
            "1",
            "--expect-reply",
            "answer — confirm?",
        ],
    );
    let (stdout, _serr, ok) = tala_in(home.path(), Some(&alpha_proj), &["pending"]);
    assert!(ok, "second pending should succeed: {}", stdout);
    assert!(
        stdout.contains("answer with"),
        "owed reply should say 'answer with --reply-to': {}",
        stdout
    );
    tala_stop(home.path());
}
