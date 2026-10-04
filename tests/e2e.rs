use serde_json::{json, Value};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use tempfile::TempDir;
fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tala"))
}
struct Harness {
    temp: TempDir,
    home: PathBuf,
    a: PathBuf,
    b: PathBuf,
}
impl Harness {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let a = temp.path().join("project-a");
        let b = temp.path().join("project-b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        Self { temp, home, a, b }
    }
    fn command(&self, dir: &Path, agent: Option<&str>, args: &[&str]) -> Command {
        let mut c = Command::new(bin());
        c.env("TALA_HOME", &self.home)
            .env_remove("TALA_AGENT")
            .current_dir(dir)
            .arg("--json");
        if let Some(id) = agent {
            c.arg(format!("--agent={id}"));
        }
        c.args(args);
        c
    }
    fn raw(&self, dir: &Path, agent: Option<&str>, args: &[&str]) -> Output {
        self.command(dir, agent, args).output().unwrap()
    }
    fn ok(&self, dir: &Path, agent: Option<&str>, args: &[&str]) -> Value {
        let o = self.raw(dir, agent, args);
        assert!(
            o.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&o.stderr)
        );
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["ok"], true);
        v["data"].clone()
    }
    fn error(&self, dir: &Path, agent: Option<&str>, args: &[&str], code: &str) -> Value {
        let o = self.raw(dir, agent, args);
        assert!(!o.status.success());
        assert!(o.stdout.is_empty());
        let v: Value = serde_json::from_slice(&o.stderr).unwrap();
        assert_eq!(v["error"]["code"], code);
        v
    }
    fn register(&self, dir: &Path, name: &str) -> String {
        self.ok(dir, None, &["register", "--name", name, "--tool=fixture"])["agent"]["id"]
            .as_str()
            .unwrap()
            .into()
    }
    fn send(&self, a: &str, b: &str, text: &str) -> Value {
        self.ok(&self.a, Some(a), &["send", "--to", b, "--request", text])
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.raw(&self.a, None, &["stop"]);
    }
}
fn id(v: &Value) -> String {
    v["message"]["id"].as_u64().unwrap().to_string()
}
fn thread(v: &Value) -> String {
    v["message"]["thread_id"].as_str().unwrap().into()
}

#[test]
fn same_checkout_agents_have_separate_inboxes_and_receipts() {
    let h = Harness::new();
    let a = h.register(&h.a, "codex");
    let b = h.register(&h.a, "claude");
    assert_ne!(a, b);
    let sent = h.send(&a, &b, "Review parser");
    let mid = id(&sent);
    assert_eq!(h.ok(&h.a, Some(&a), &["inbox"])["messages"], json!([]));
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox", "--peek"])["messages"][0]["id"],
        sent["message"]["id"]
    );
    h.ok(&h.a, Some(&b), &["history", &thread(&sent)]);
    assert_eq!(
        h.ok(&h.a, Some(&a), &["status", "--message", &mid])["message"]["delivery"],
        "stored"
    );
    h.ok(&h.a, Some(&b), &["inbox"]);
    assert_eq!(
        h.ok(&h.a, Some(&a), &["status", "--message", &mid])["message"]["delivery"],
        "received"
    );
    assert_eq!(
        h.ok(&h.a, Some(&b), &["pending", "--direction=incoming"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let reply = h.ok(&h.a, Some(&b), &["reply", &mid, "Two issues"]);
    assert_eq!(reply["message"]["recipient"], a);
    assert_eq!(reply["message"]["thread_id"], sent["message"]["thread_id"]);
    assert_eq!(
        h.ok(&h.a, Some(&a), &["status", "--message", &mid])["message"]["delivery"],
        "answered"
    );
    assert_eq!(h.ok(&h.a, Some(&a), &["pending"])["messages"], json!([]));
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox"])["messages"][0]["reply_to"],
        sent["message"]["id"]
    );
}
#[test]
fn cross_project_context_boards_and_handoff() {
    let h = Harness::new();
    let a = h.register(&h.a, "frontend");
    let b = h.register(&h.b, "api");
    let c = h.register(&h.b, "reviewer");
    let sent = h.send(&a, &b, "Does null work?");
    let t = thread(&sent);
    assert_eq!(
        sent["message"]["context"]["project"],
        std::fs::canonicalize(&h.a).unwrap().display().to_string()
    );
    h.error(&h.b, Some(&c), &["history", &t], "THREAD_NOT_VISIBLE");
    h.ok(&h.a, Some(&a), &["post", "Frontend update"]);
    assert_eq!(h.ok(&h.b, Some(&b), &["board"])["messages"], json!([]));
    assert_eq!(
        h.ok(
            &h.b,
            Some(&b),
            &["board", "--project", h.a.to_str().unwrap()]
        )["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    h.ok(
        &h.b,
        Some(&b),
        &[
            "handoff",
            "--to",
            &c,
            "--thread",
            &t,
            "Please review API contract",
        ],
    );
    assert_eq!(
        h.ok(&h.b, Some(&c), &["history", &t])["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        h.ok(&h.b, Some(&b), &["inbox", "--peek"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn duplicate_names_unknown_recipients_and_explicit_identity() {
    let h = Harness::new();
    let a = h.register(&h.a, "codex");
    let b = h.register(&h.a, "codex");
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to=codex", "hi"],
        "AMBIGUOUS_RECIPIENT",
    );
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to=missing", "hi"],
        "RECIPIENT_NOT_FOUND",
    );
    h.error(&h.a, None, &["inbox"], "IDENTITY_REQUIRED");
    h.send(&a, &b, "hi");
    let nested = h.a.join("src");
    std::fs::create_dir(&nested).unwrap();
    assert_eq!(h.ok(&nested, Some(&a), &["whoami"])["agent"]["id"], a);
}
#[test]
fn bounded_inbox_does_not_skip_messages_or_touch_peer() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let c = h.register(&h.a, "c");
    let one = h.send(&a, &b, "one");
    h.send(&a, &c, "other");
    h.send(&a, &b, "two");
    let page = h.ok(&h.a, Some(&b), &["inbox", "--limit=1"]);
    assert_eq!(page["messages"][0]["id"], one["message"]["id"]);
    assert_eq!(page["has_more"], true);
    let after = page["next_after"].as_u64().unwrap().to_string();
    let page = h.ok(&h.a, Some(&b), &["inbox", "--after", &after]);
    assert_eq!(page["messages"].as_array().unwrap().len(), 1);
    assert_eq!(
        h.ok(&h.a, Some(&c), &["inbox"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn concurrent_inbox_consumers_receive_disjoint_batches() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.send(&a, &b, "one");
    h.send(&a, &b, "two");
    let first = h
        .command(&h.a, Some(&b), &["inbox", "--limit=1"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let second = h
        .command(&h.a, Some(&b), &["inbox", "--limit=1"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let read = |o: Output| -> Value {
        assert!(o.status.success());
        serde_json::from_slice::<Value>(&o.stdout).unwrap()["data"]["messages"][0]["id"].clone()
    };
    assert_ne!(
        read(first.wait_with_output().unwrap()),
        read(second.wait_with_output().unwrap())
    );
}
#[test]
fn concurrent_startup_has_one_owner() {
    let h = Harness::new();
    let mut children = Vec::new();
    for n in 0..6 {
        children.push(
            h.command(
                &h.a,
                None,
                &["register", "--name", &format!("a{n}"), "--tool=fixture"],
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
        );
    }
    for child in children {
        let o = child.wait_with_output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let status = h.ok(&h.a, None, &["status"]);
    assert_eq!(status["agents"], 6);
    let o = h.command(&h.a, None, &["daemon"]).output().unwrap();
    assert!(!o.status.success());
    assert_eq!(h.ok(&h.a, None, &["status"])["agents"], 6);
}
#[test]
fn restart_preserves_messages_receipts_and_dedup() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let args = ["send", "--to", &b, "--key=stable", "persistent"];
    let one = h.ok(&h.a, Some(&a), &args);
    h.ok(&h.b, Some(&b), &["inbox"]);
    h.ok(&h.a, None, &["stop"]);
    let two = h.ok(&h.a, Some(&a), &args);
    assert_eq!(two["duplicate"], true);
    assert_eq!(one["message"]["id"], two["message"]["id"]);
    assert_eq!(h.ok(&h.b, Some(&b), &["inbox"])["messages"], json!([]));
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to", &b, "--key=stable", "changed"],
        "IDEMPOTENCY_CONFLICT",
    );
}
#[test]
fn request_cancel_and_follow_up_have_explicit_semantics() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let req = h.send(&a, &b, "question");
    h.ok(
        &h.a,
        Some(&a),
        &[
            "send",
            "--to",
            &b,
            "--thread",
            &thread(&req),
            "--request",
            "followup",
        ],
    );
    assert_eq!(
        h.ok(&h.a, Some(&a), &["pending", "--direction=outgoing"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    h.error(&h.a, Some(&b), &["resolve", &id(&req)], "INVALID_OPERATION");
    h.ok(&h.a, Some(&a), &["resolve", &id(&req)]);
    assert_eq!(
        h.ok(&h.a, Some(&a), &["pending"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn timeout_retains_stored_request_and_late_reply() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let o = h.raw(
        &h.a,
        Some(&a),
        &["send", "--to", &b, "--wait", "--timeout=1", "question"],
    );
    assert_eq!(o.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let sent = &v["data"]["sent"];
    assert_eq!(v["data"]["timed_out"], true);
    assert_eq!(v["data"]["sent"]["snapshot"], "at_send");
    h.ok(&h.b, Some(&b), &["reply", &id(sent), "late reply"]);
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox"])["messages"][0]["parts"][0]["text"],
        "late reply"
    );
}
#[test]
fn blocking_send_matches_only_correlated_reply_and_preserves_other_messages() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let waiter = h
        .command(
            &h.a,
            Some(&a),
            &["send", "--to", &b, "--wait", "--timeout=5", "question"],
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let mid = loop {
        let messages = h.ok(&h.b, Some(&b), &["inbox", "--peek"]);
        if let Some(m) = messages["messages"].as_array().unwrap().first() {
            break m["id"].as_u64().unwrap().to_string();
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(50));
    };
    h.ok(&h.b, Some(&b), &["send", "--to", &a, "unrelated"]);
    h.ok(&h.b, Some(&b), &["reply", &mid, "answer"]);
    let o = waiter.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["messages"][0]["parts"][0]["text"], "answer");
    assert_eq!(v["data"]["sent"]["message"]["delivery"], "answered");
    assert_eq!(v["data"]["sent"]["snapshot"], "at_reply_receipt");
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox"])["messages"][0]["parts"][0]["text"],
        "unrelated"
    );
}
#[test]
fn board_search_history_are_nonconsuming_and_parts_are_preserved() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let sent = h.ok(
        &h.a,
        Some(&a),
        &[
            "send",
            "--to",
            &b,
            "--part=text:parser",
            "--part=file:src/lib.rs",
            "--part=data:{\"ok\":true}",
        ],
    );
    h.ok(&h.a, Some(&a), &["post", "parser ready"]);
    let board = h.ok(&h.a, Some(&b), &["board"]);
    assert_eq!(board["messages"].as_array().unwrap().len(), 1);
    h.ok(
        &h.a,
        Some(&b),
        &[
            "reply",
            &board["messages"][0]["id"].as_u64().unwrap().to_string(),
            "noted",
        ],
    );
    let search = h.ok(&h.a, Some(&b), &["search", "parser"]);
    assert_eq!(search["messages"].as_array().unwrap().len(), 2);
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox"])["messages"][0]["parts"],
        sent["message"]["parts"]
    );
}
#[test]
fn multiline_stdin_file_and_usage_errors() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let text = "Code: `x`\n$literal 'quoted'\n";
    let mut child = h
        .command(&h.a, Some(&a), &["send", "--to", &b, "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["message"]["parts"][0]["text"], text);
    let file = h.temp.path().join("draft.txt");
    std::fs::write(&file, text).unwrap();
    assert_eq!(
        h.ok(
            &h.a,
            Some(&a),
            &["send", "--to", &b, "--message-file", file.to_str().unwrap()]
        )["message"]["parts"][0]["text"],
        text
    );
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to", &b, "hello", "--stdin"],
        "USAGE_ERROR",
    );
    h.error(&h.a, Some(&a), &["inbox", "--limit=0"], "USAGE_ERROR");
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to", &b, "--part=text:"],
        "USAGE_ERROR",
    );
}
#[test]
fn offline_presence_and_legacy_preservation() {
    let h = Harness::new();
    std::fs::create_dir_all(&h.home).unwrap();
    let legacy = h.home.join("messages.json");
    std::fs::write(&legacy, "legacy").unwrap();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.ok(&h.a, Some(&b), &["unregister"]);
    h.send(&a, &b, "offline");
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(std::fs::read_to_string(&legacy).unwrap(), "legacy");
    assert_eq!(
        h.ok(&h.a, None, &["status"])["legacy_files"],
        json!(["messages.json"])
    );
}
#[test]
fn status_is_read_only_and_corrupt_database_fails_loudly() {
    let h = Harness::new();
    assert_eq!(h.ok(&h.a, None, &["status"])["running"], false);
    assert!(!h.home.exists());
    std::fs::create_dir(&h.home).unwrap();
    std::fs::write(h.home.join("tala.sqlite3"), "not a database").unwrap();
    let o = h.raw(&h.a, None, &["daemon"]);
    assert!(!o.status.success());
    let v: Value = serde_json::from_slice(&o.stderr).unwrap();
    assert_eq!(v["error"]["code"], "STORAGE_ERROR");
    assert_eq!(
        std::fs::read_to_string(h.home.join("tala.sqlite3")).unwrap(),
        "not a database"
    );
}
#[test]
fn integration_generation_is_safe_and_versioned() {
    let h = Harness::new();
    std::fs::create_dir(h.a.join(".opencode")).unwrap();
    h.ok(&h.a, None, &["init", "--dry-run"]);
    assert!(!h.a.join(".tala").exists());
    h.ok(&h.a, None, &["init"]);
    let guide = h.a.join(".tala/AGENTS.md");
    let content = std::fs::read_to_string(&guide).unwrap();
    assert!(content.contains(env!("CARGO_PKG_VERSION")));
    assert!(content.contains("Peer messages"));
    std::fs::write(&guide, "custom").unwrap();
    h.ok(&h.a, None, &["init"]);
    assert_eq!(std::fs::read_to_string(&guide).unwrap(), "custom");
    h.ok(&h.a, None, &["init", "--check"]);
    assert_eq!(std::fs::read_to_string(&guide).unwrap(), "custom");
    h.ok(&h.a, None, &["init", "--refresh"]);
    assert_eq!(std::fs::read_to_string(&guide).unwrap(), content);
    assert!(!h.a.join(".tala/config.json").exists());
    assert!(!h.a.join(".tala/active-session").exists());
}
#[test]
fn new_help_rejects_old_surface() {
    let h = Harness::new();
    let o = Command::new(bin()).arg("--help").output().unwrap();
    let help = String::from_utf8(o.stdout).unwrap();
    for cmd in ["register", "inbox", "reply", "board", "handoff"] {
        assert!(help.contains(cmd));
    }
    for cmd in ["use", "session", "check", "listen", "discover"] {
        h.error(&h.a, None, &[cmd], "USAGE_ERROR");
    }
    let o = Command::new(bin()).arg("--version").output().unwrap();
    assert_eq!(
        String::from_utf8(o.stdout).unwrap().trim(),
        format!("tala {}", env!("CARGO_PKG_VERSION"))
    );
}
#[cfg(unix)]
#[test]
fn abrupt_crash_keeps_committed_messages() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let sent = h.send(&a, &b, "durable");
    let info: Value =
        serde_json::from_slice(&std::fs::read(h.home.join("daemon.json")).unwrap()).unwrap();
    let pid = info["pid"].as_u64().unwrap();
    let status = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        h.ok(&h.b, Some(&b), &["inbox"])["messages"][0]["id"],
        sent["message"]["id"]
    );
}

#[test]
fn direct_messages_cannot_leak_into_public_board_threads() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let post = h.ok(&h.a, Some(&a), &["post", "public"]);
    h.error(
        &h.a,
        Some(&a),
        &["send", "--to", &b, "--thread", &thread(&post), "private"],
        "INVALID_OPERATION",
    );
    h.error(
        &h.a,
        Some(&b),
        &["reply", &id(&post), "--request", "question"],
        "INVALID_OPERATION",
    );
}
#[test]
fn explicit_reply_acknowledges_original_and_rejects_impersonation() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let c = h.register(&h.a, "c");
    let sent = h.send(&a, &b, "question");
    h.error(
        &h.a,
        Some(&c),
        &["reply", &id(&sent), "wrong"],
        "NOT_RECIPIENT",
    );
    h.ok(&h.a, Some(&b), &["reply", &id(&sent), "answer"]);
    assert_eq!(h.ok(&h.a, Some(&b), &["inbox"])["messages"], json!([]));
}
#[test]
fn inbox_wait_wakes_and_infinite_wait_can_be_interrupted() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let waiter = h
        .command(&h.a, Some(&b), &["inbox", "--wait", "--timeout=0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let message = h.send(&a, &b, "wake");
    let output = waiter.wait_with_output().unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["data"]["messages"][0]["id"], message["message"]["id"]);
    let mut waiter = h
        .command(&h.a, Some(&b), &["inbox", "--wait", "--timeout=0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(waiter.try_wait().unwrap().is_none());
    waiter.kill().unwrap();
    waiter.wait().unwrap();
}
#[test]
fn git_root_and_branch_context_are_stable_from_nested_directories() {
    let h = Harness::new();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&h.a)
        .status()
        .unwrap()
        .success());
    let nested = h.a.join("src");
    std::fs::create_dir(&nested).unwrap();
    let a = h.register(&nested, "a");
    let identity = h.ok(&nested, Some(&a), &["whoami"]);
    assert_eq!(
        identity["agent"]["context"]["project"],
        std::fs::canonicalize(&h.a).unwrap().display().to_string()
    );
    assert!(identity["agent"]["context"]["branch"].is_string());
    h.ok(&nested, None, &["init"]);
    assert!(h.a.join(".tala/AGENTS.md").exists());
    assert!(!nested.join(".tala").exists());
}

#[test]
fn incompatible_live_legacy_daemon_blocks_registration_without_new_storage() {
    use std::io::Read;
    use std::net::TcpListener;
    let h = Harness::new();
    std::fs::create_dir(&h.home).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        h.home.join("daemon.json"),
        serde_json::to_vec(&json!({"pid":42,"port":port,"protocol_version":1})).unwrap(),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).unwrap();
        let body = "{\"pid\":42,\"protocol_version\":1}";
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    h.error(
        &h.a,
        None,
        &["register", "--name=new", "--tool=test"],
        "PROTOCOL_MISMATCH",
    );
    server.join().unwrap();
    assert!(!h.home.join("tala.sqlite3").exists());
}

#[test]
fn integration_preflight_failure_preserves_existing_documents() {
    let h = Harness::new();
    std::fs::create_dir_all(h.a.join(".opencode/commands/tala.md")).unwrap();
    std::fs::create_dir_all(h.a.join(".tala")).unwrap();
    std::fs::write(h.a.join(".tala/AGENTS.md"), "original").unwrap();
    h.error(&h.a, None, &["init", "--refresh"], "IO_ERROR");
    assert_eq!(
        std::fs::read_to_string(h.a.join(".tala/AGENTS.md")).unwrap(),
        "original"
    );
    assert!(!h.a.join(".opencode/skills/tala/SKILL.md").exists());
}
