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
/// A registered agent: public ID for addressing, private handle for acting.
#[derive(Clone, Debug, PartialEq)]
struct Ident {
    id: String,
    handle: String,
}
impl std::ops::Deref for Ident {
    type Target = str;
    fn deref(&self) -> &str {
        &self.id
    }
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
    fn command(&self, dir: &Path, agent: Option<&Ident>, args: &[&str]) -> Command {
        let mut c = Command::new(bin());
        c.env("TALA_HOME", &self.home)
            .env_remove("TALA_AGENT")
            .env_remove("TALA_AGENT_FILE")
            .current_dir(dir)
            .arg("--json");
        if let Some(agent) = agent {
            c.arg(format!("--agent={}", agent.handle));
        }
        c.args(args);
        c
    }
    fn raw(&self, dir: &Path, agent: Option<&Ident>, args: &[&str]) -> Output {
        self.command(dir, agent, args).output().unwrap()
    }
    fn ok(&self, dir: &Path, agent: Option<&Ident>, args: &[&str]) -> Value {
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
    fn error(&self, dir: &Path, agent: Option<&Ident>, args: &[&str], code: &str) -> Value {
        let o = self.raw(dir, agent, args);
        assert!(!o.status.success());
        assert!(o.stdout.is_empty());
        let v: Value = serde_json::from_slice(&o.stderr).unwrap();
        assert_eq!(v["error"]["code"], code);
        v
    }
    fn register(&self, dir: &Path, name: &str) -> Ident {
        let v = self.ok(dir, None, &["register", "--name", name, "--tool=fixture"]);
        Ident {
            id: v["agent"]["id"].as_str().unwrap().into(),
            handle: v["handle"].as_str().unwrap().into(),
        }
    }
    fn send(&self, a: &Ident, b: &str, text: &str) -> Value {
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
    assert_eq!(reply["message"]["recipient"], a.id);
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
fn human_send_explains_that_an_unlistened_message_stays_queued() {
    let h = Harness::new();
    let sender = h.register(&h.a, "sender");
    let recipient = h.register(&h.b, "recipient");
    let output = h
        .command(
            &h.a,
            Some(&sender),
            &["send", "--to", &recipient.id, "--request", "Please review"],
        )
        .arg("--human")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Recipient was not listening when Tala accepted the message."));
    assert!(stdout.contains("The message remains queued."));
}

#[test]
fn human_reply_explains_that_an_unlistened_message_stays_queued() {
    let h = Harness::new();
    let sender = h.register(&h.a, "sender");
    let recipient = h.register(&h.b, "recipient");
    let request = h.send(&sender, &recipient.id, "Please review");
    let message_id = request["message"]["id"].as_u64().unwrap().to_string();
    let output = h
        .command(
            &h.b,
            Some(&recipient),
            &["reply", &message_id, "Review complete"],
        )
        .arg("--human")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Recipient was not listening when Tala accepted the message."));
    assert!(stdout.contains("The message remains queued."));
}

#[test]
fn project_list_hides_closed_projects_and_owner_can_reopen() {
    let h = Harness::new();
    let owner = h.register(&h.a, "owner");
    let peer = h.register(&h.a, "peer");
    let other = h.register(&h.b, "other");
    let project_a = std::fs::canonicalize(&h.a).unwrap().display().to_string();
    let project_b = std::fs::canonicalize(&h.b).unwrap().display().to_string();

    let initial = h.ok(&h.a, Some(&owner), &["list-projects"]);
    assert_eq!(initial["projects"].as_array().unwrap().len(), 2);
    assert_eq!(initial["projects"][0]["project"], project_a);
    assert_eq!(initial["projects"][0]["owner_id"], owner.id);
    assert_eq!(initial["projects"][1]["project"], project_b);
    assert_eq!(initial["projects"][1]["owner_id"], other.id);

    let closed = h.ok(&h.a, Some(&owner), &["close-project"]);
    assert_eq!(closed["closed"], true);
    assert_eq!(closed["changed"], true);
    let default = h.ok(&h.a, Some(&owner), &["list-projects"]);
    assert_eq!(default["projects"].as_array().unwrap().len(), 1);
    assert_eq!(default["projects"][0]["project"], project_b);
    assert_eq!(default["hidden"]["closed"], 1);

    let error = h.error(&h.a, Some(&peer), &["close-project"], "NOT_PROJECT_OWNER");
    assert_eq!(error["error"]["retry"], "never");
    let with_closed = h.ok(&h.a, Some(&owner), &["list-projects", "--closed"]);
    assert_eq!(with_closed["projects"].as_array().unwrap().len(), 2);
    assert_eq!(with_closed["projects"][0]["closed"], true);

    let reopened = h.ok(&h.a, Some(&owner), &["reopen-project"]);
    assert_eq!(reopened["closed"], false);
    let all = h.ok(&h.a, Some(&owner), &["list-projects", "--all"]);
    assert_eq!(all["projects"].as_array().unwrap().len(), 2);
}

#[test]
fn project_list_works_without_identity_and_keeps_authenticated_attention() {
    let h = Harness::new();

    let empty = h.raw(&h.a, None, &["list-projects"]);
    assert!(
        empty.status.success(),
        "{}",
        String::from_utf8_lossy(&empty.stderr)
    );
    let empty: Value = serde_json::from_slice(&empty.stdout).unwrap();
    assert_eq!(empty["ok"], true);
    assert_eq!(empty["data"]["projects"], json!([]));
    assert!(empty.get("attention").is_none());

    let owner = h.register(&h.a, "owner");
    let peer = h.register(&h.b, "peer");
    let listed = h.ok(&h.a, None, &["list-projects"]);
    assert_eq!(listed["projects"].as_array().unwrap().len(), 2);

    h.send(&peer, &owner.id, "Review this project");
    let authenticated = h.raw(&h.a, Some(&owner), &["list-projects"]);
    assert!(authenticated.status.success());
    let authenticated: Value = serde_json::from_slice(&authenticated.stdout).unwrap();
    assert_eq!(authenticated["ok"], true);
    assert_eq!(authenticated["attention"]["open_requests"], 1);
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
    assert_eq!(h.ok(&nested, Some(&a), &["whoami"])["agent"]["id"], a.id);
}

#[test]
fn rename_preserves_identity_threads_and_duplicate_name_behavior() {
    let h = Harness::new();
    let sender = h.register(&h.a, "codex");
    let target = h.register(&h.a, "reviewer");
    h.error(&h.a, None, &["rename", "--name=other"], "IDENTITY_REQUIRED");

    let direct = h.send(&sender, &target, "Review this change");
    let direct_thread = thread(&direct);
    let board = h.ok(&h.a, Some(&target), &["post", "Shared project note"]);
    let board_thread = thread(&board);

    let renamed = h.ok(&h.a, Some(&target), &["rename", "--name=codex"]);
    assert_eq!(renamed["agent"]["id"], target.id);
    assert_eq!(renamed["agent"]["name"], "codex");
    assert_eq!(renamed["previous_name"], "reviewer");
    assert_eq!(renamed["changed"], true);

    let whoami = h.ok(&h.a, Some(&target), &["whoami"]);
    assert_eq!(whoami["agent"]["id"], target.id);
    assert_eq!(whoami["agent"]["name"], "codex");
    for history_thread in [&direct_thread, &board_thread] {
        let history = h.ok(&h.a, Some(&target), &["history", history_thread]);
        assert!(!history["messages"].as_array().unwrap().is_empty());
    }

    // The original credential remains valid, and exact IDs remain unambiguous.
    let addressed = h.send(&sender, &target, "Still the same identity");
    assert_eq!(addressed["message"]["recipient"], target.id);
    h.error(
        &h.a,
        Some(&sender),
        &["send", "--to=codex", "Ambiguous by name"],
        "AMBIGUOUS_RECIPIENT",
    );

    let no_op = h.ok(&h.a, Some(&target), &["rename", "--name=codex"]);
    assert_eq!(no_op["agent"]["id"], target.id);
    assert_eq!(no_op["changed"], false);
    assert_eq!(no_op["previous_name"], "codex");
    let human = h.raw(&h.a, Some(&target), &["rename", "--name=codex", "--human"]);
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).contains("already named codex"));
    h.error(
        &h.a,
        Some(&target),
        &["rename", "--name="],
        "INVALID_OPERATION",
    );
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
    assert_eq!(
        v["data"]["next"],
        format!("tala inbox --wait --reply-to={} --timeout=1", id(sent))
    );
    assert_eq!(
        h.ok(&h.a, Some(&a), &["status", "--message", &id(sent)])["message"]["delivery"],
        "stored"
    );
    h.ok(&h.b, Some(&b), &["reply", &id(sent), "late reply"]);
    assert_eq!(
        h.ok(
            &h.a,
            Some(&a),
            &["inbox", "--wait", "--reply-to", &id(sent), "--timeout=5"]
        )["messages"][0]["parts"][0]["text"],
        "late reply"
    );
    assert_eq!(
        h.ok(&h.a, Some(&a), &["status", "--message", &id(sent)])["message"]["delivery"],
        "answered"
    );
}

#[test]
fn send_wait_timeout_human_output_has_safe_recovery_guidance() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let o = h.raw(
        &h.a,
        Some(&a),
        &[
            "send",
            "--to",
            &b,
            "--wait",
            "--timeout=1",
            "question",
            "--human",
        ],
    );
    assert_eq!(o.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains("the request was stored. Do not resend it."));
    assert!(stdout.contains("tala inbox --wait --reply-to=1 --timeout=1"));
    assert!(stdout.contains("The message remains queued."));
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
    for cmd in ["register", "rename", "inbox", "reply", "board", "handoff"] {
        assert!(help.contains(cmd));
    }
    for cmd in ["use", "session", "check", "listen", "discover"] {
        h.error(&h.a, None, &[cmd], "USAGE_ERROR");
    }
    let register_help = Command::new(bin())
        .args(["register", "--help"])
        .output()
        .unwrap();
    let register_help = String::from_utf8(register_help.stdout).unwrap();
    assert!(register_help.contains("--save-handle-to"));
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
fn unified_wait_reports_direct_messages_and_replies_without_consuming_them() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let request = h.ok(
        &h.a,
        Some(&a),
        &["send", "--to", &b.id, "--request", "question"],
    );
    h.ok(&h.a, Some(&b), &["reply", &id(&request), "answer"]);
    h.ok(&h.a, Some(&b), &["send", "--to", &a.id, "additional note"]);
    h.ok(&h.a, Some(&b), &["post", "parallel board update"]);

    let waiting = h.ok(&h.a, Some(&a), &["wait", "--board-after=0", "--timeout=2"]);
    let events = waiting["events"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["type"], "direct_reply");
    assert_eq!(events[1]["type"], "direct_message");
    assert_eq!(events[2]["type"], "board_post");
    assert!(!waiting.to_string().contains("answer"));
    assert!(waiting["next"]["inbox"]
        .as_str()
        .unwrap()
        .contains("inbox --peek"));
    assert!(waiting["next"]["listen_again"]
        .as_str()
        .unwrap()
        .contains("--inbox-after="));
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox", "--peek"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    h.ok(&h.a, Some(&a), &["inbox"]);

    let mut waiter = h
        .command(&h.a, Some(&a), &["wait"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(waiter.try_wait().unwrap().is_none());
    h.ok(
        &h.a,
        Some(&b),
        &["send", "--to", &a.id, "wake the background wait"],
    );
    let output = waiter.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["data"]["events"][0]["type"], "direct_message");
}

#[test]
fn unified_wait_deduplicates_board_replies_and_reports_both_sources() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let post = h.ok(&h.a, Some(&a), &["post", "plan"]);
    let post_id = id(&post);
    let reply = h.ok(&h.a, Some(&b), &["reply", &post_id, "reviewed"]);
    let reply_id = id(&reply);

    let waiting = h.ok(
        &h.a,
        Some(&a),
        &[
            "wait",
            "--inbox-after=0",
            "--board-after",
            &post_id,
            "--timeout=2",
        ],
    );
    let events = waiting["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["id"], reply_id.parse::<u64>().unwrap());
    assert_eq!(events[0]["type"], "board_reply");
    assert_eq!(events[0]["sources"], json!(["inbox", "board"]));
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox", "--peek"])["messages"][0]["id"],
        reply_id.parse::<u64>().unwrap()
    );
}

#[test]
fn unified_wait_watches_explicit_project_and_resumes_from_separate_cursors() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let post = h.ok(&h.b, Some(&b), &["post", "other project update"]);
    let post_id = id(&post);
    let project = "../project-b";
    let canonical_project = h.b.canonicalize().unwrap().display().to_string();
    let waiting = h.ok(
        &h.a,
        Some(&a),
        &[
            "wait",
            "--project",
            project,
            "--inbox-after=0",
            "--board-after=0",
            "--timeout=2",
        ],
    );
    assert_eq!(waiting["events"][0]["id"], post_id.parse::<u64>().unwrap());
    assert_eq!(waiting["events"][0]["type"], "board_post");
    let restart = waiting["next"]["listen_again"].as_str().unwrap();
    assert!(restart.contains("--project="));
    assert!(restart.contains(&canonical_project));
    assert!(restart.contains(&format!("--board-after={post_id}")));
    assert!(restart.contains("--inbox-after=0"));

    let inbox_sender = h.register(&h.a, "sender");
    h.ok(
        &h.a,
        Some(&inbox_sender),
        &["send", "--to", &a.id, "later note"],
    );
    let after_id = waiting["inbox_after"].as_u64().unwrap();
    let later = h.ok(
        &h.a,
        Some(&a),
        &[
            "wait",
            "--project",
            project,
            &format!("--inbox-after={after_id}"),
            &format!("--board-after={post_id}"),
            "--timeout=2",
        ],
    );
    assert_eq!(later["events"][0]["type"], "direct_message");
}

#[test]
fn unified_wait_baselines_existing_board_posts_and_times_out_cleanly() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let post = h.ok(&h.a, Some(&b), &["post", "already there"]);
    let _post_id = id(&post);
    h.ok(&h.a, Some(&b), &["post", "also already there"]);
    let latest_post = h.ok(&h.a, Some(&b), &["post", "latest already there"]);
    let board_page = h.ok(&h.a, Some(&a), &["board", "--limit=1"]);
    assert_eq!(board_page["has_more"], true);
    assert_eq!(board_page["latest_id"], latest_post["message"]["id"]);
    let output = h.raw(&h.a, Some(&a), &["wait", "--timeout=1"]);
    assert_eq!(output.status.code(), Some(3));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["data"]["events"], json!([]));
    assert_eq!(response["data"]["timed_out"], true);
    assert_eq!(
        response["data"]["board_after"],
        latest_post["message"]["id"]
    );
    assert!(response["data"]["next"]["listen_again"]
        .as_str()
        .unwrap()
        .contains(&format!("--board-after={}", latest_post["message"]["id"])));

    let waiter = h
        .command(&h.a, Some(&a), &["wait", "--timeout=5"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let new_post = h.ok(&h.a, Some(&b), &["post", "arrived after startup"]);
    let output = waiter.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        response["data"]["events"][0]["id"],
        new_post["message"]["id"]
    );
    assert_eq!(response["data"]["events"][0]["type"], "board_post");
}

#[test]
fn unified_wait_cursor_paginates_unread_messages_without_skipping() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    for i in 0..51 {
        let text = format!("note {i}");
        h.ok(&h.a, Some(&b), &["send", "--to", &a.id, &text]);
    }

    let first = h.ok(
        &h.a,
        Some(&a),
        &["wait", "--inbox-after=0", "--board-after=0", "--timeout=2"],
    );
    assert_eq!(first["events"].as_array().unwrap().len(), 50);
    assert_eq!(first["inbox_has_more"], true);
    let cursor = first["inbox_after"].as_u64().unwrap();
    assert_eq!(cursor, first["events"][49]["id"].as_u64().unwrap());

    let second = h.ok(
        &h.a,
        Some(&a),
        &[
            "wait",
            &format!("--inbox-after={cursor}"),
            "--board-after=0",
            "--timeout=2",
        ],
    );
    assert_eq!(second["events"].as_array().unwrap().len(), 1);
    assert_eq!(second["events"][0]["id"].as_u64().unwrap(), cursor + 1);
    assert_eq!(h.ok(&h.a, Some(&a), &[])["unread"]["count"], 51);
}

#[test]
fn unified_wait_board_cursor_paginates_without_skipping_posts() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    for i in 0..51 {
        let text = format!("board update {i}");
        h.ok(&h.a, Some(&b), &["post", &text]);
    }

    let first = h.ok(
        &h.a,
        Some(&a),
        &["wait", "--inbox-after=0", "--board-after=0", "--timeout=2"],
    );
    assert_eq!(first["events"].as_array().unwrap().len(), 50);
    assert_eq!(first["board_has_more"], true);
    let cursor = first["board_after"].as_u64().unwrap();
    assert_eq!(cursor, first["events"][49]["id"].as_u64().unwrap());

    let second = h.ok(
        &h.a,
        Some(&a),
        &[
            "wait",
            "--inbox-after=0",
            &format!("--board-after={cursor}"),
            "--timeout=2",
        ],
    );
    assert_eq!(second["events"].as_array().unwrap().len(), 1);
    assert_eq!(second["events"][0]["id"].as_u64().unwrap(), cursor + 1);
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
fn outgoing_message_records_current_directory_without_changing_identity_context() {
    let h = Harness::new();
    git(&h.a, &["init", "-q"]);
    git(&h.a, &["branch", "-M", "main"]);
    git(
        &h.a,
        &["commit", "-q", "--allow-empty", "-m", "registered revision"],
    );
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let sent = h.ok(
        &h.b,
        Some(&a),
        &["send", "--to", &b, "message from another checkout"],
    );
    assert_eq!(
        sent["message"]["context"]["project"],
        h.a.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(
        sent["message"]["context"]["working_directory"],
        h.b.canonicalize().unwrap().display().to_string()
    );
    assert!(sent["message"]["context"]["worktree"].is_null());
    assert!(sent["message"]["context"]["branch"].is_null());
    assert!(sent["message"]["context"]["revision"].is_null());
    assert_eq!(
        h.ok(&h.b, Some(&a), &["whoami"])["agent"]["context"]["project"],
        h.a.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(
        h.ok(&h.b, Some(&a), &["whoami"])["agent"]["context"]["branch"],
        "main"
    );
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
        // The CLI first opens a bare connection to classify the port; answer
        // the first connection that sends a request.
        let mut stream = loop {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            if stream.read(&mut request).unwrap_or(0) > 0 {
                break stream;
            }
        };
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
fn status_reports_an_incompatible_daemon_instead_of_failing() {
    use std::io::Read;
    use std::net::TcpListener;
    let h = Harness::new();
    std::fs::create_dir(&h.home).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(
        h.home.join("daemon.json"),
        json!({"pid":42,"port":port,"protocol_version":2,"instance":"old"}).to_string(),
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        // The CLI first opens a bare connection to classify the port; answer
        // the first connection that sends a request.
        let mut stream = loop {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            if stream.read(&mut request).unwrap_or(0) > 0 {
                break stream;
            }
        };
        let body = "{\"instance\":\"old\",\"protocol_version\":2}";
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    let status = h.ok(&h.a, None, &["status"]);
    server.join().unwrap();
    assert_eq!(status["compatible"], false);
    assert_eq!(status["error"]["code"], "PROTOCOL_MISMATCH");
    std::fs::remove_file(h.home.join("daemon.json")).unwrap();
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

#[cfg(unix)]
fn daemon_pid(h: &Harness) -> u64 {
    let info: Value =
        serde_json::from_slice(&std::fs::read(h.home.join("daemon.json")).unwrap()).unwrap();
    info["pid"].as_u64().unwrap()
}
/// Pauses the daemon so it stays alive but cannot answer; resumes on drop.
#[cfg(unix)]
struct Paused(u64);
#[cfg(unix)]
impl Paused {
    fn new(h: &Harness) -> Self {
        let pid = daemon_pid(h);
        assert!(Command::new("kill")
            .args(["-STOP", &pid.to_string()])
            .status()
            .unwrap()
            .success());
        Paused(pid)
    }
}
#[cfg(unix)]
impl Drop for Paused {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-CONT", &self.0.to_string()])
            .status();
    }
}
#[cfg(unix)]
fn second_daemon_attempted(h: &Harness) -> bool {
    std::fs::read_to_string(h.home.join("daemon.log"))
        .unwrap_or_default()
        .contains("already owns")
}
#[cfg(unix)]
fn wait_for_message(h: &Harness, agent: &Ident) -> String {
    let start = Instant::now();
    loop {
        let messages = h.ok(&h.b, Some(agent), &["inbox", "--peek"]);
        if let Some(m) = messages["messages"].as_array().unwrap().first() {
            return m["id"].as_u64().unwrap().to_string();
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn stderr_json(o: &Output) -> Value {
    serde_json::from_slice(&o.stderr).unwrap()
}

#[test]
fn help_lists_exit_codes_and_every_error_code() {
    let o = Command::new(bin()).arg("--help").output().unwrap();
    let help = String::from_utf8(o.stdout).unwrap();
    assert!(help.contains("4 stored but wait failed"));
    for code in [
        "WAIT_FAILED",
        "DAEMON_UNREACHABLE",
        "RECIPIENT_NOT_FOUND",
        "NOT_PROJECT_OWNER",
        "same_key",
        "list-projects",
        "close-project",
        "reopen-project",
        "--stale",
        "--closed",
        "--all",
        "--agent-file",
        "TALA_AGENT_FILE",
    ] {
        assert!(help.contains(code), "{code}");
    }
}
#[test]
fn errors_carry_retry_class_stored_and_next() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let e = h.error(
        &h.a,
        Some(&a),
        &["send", "--to=missing", "hi"],
        "RECIPIENT_NOT_FOUND",
    );
    assert_eq!(e["error"]["retry"], "after_fix");
    assert_eq!(e["error"]["next"], "tala agents");
    assert!(e["error"]["stored"].is_null());
}
#[test]
fn refused_metadata_is_replaced_by_a_new_daemon() {
    let h = Harness::new();
    std::fs::create_dir(&h.home).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::fs::write(
        h.home.join("daemon.json"),
        json!({"pid":1,"port":port,"protocol_version":3,"instance":"gone"}).to_string(),
    )
    .unwrap();
    assert_eq!(h.ok(&h.a, None, &["status"])["stale_metadata"], true);
    h.register(&h.a, "a");
    assert_eq!(h.ok(&h.a, None, &["status"])["running"], true);
}
#[test]
fn unreachable_daemon_is_reported_and_never_replaced() {
    let h = Harness::new();
    std::fs::create_dir(&h.home).unwrap();
    // Accepts connections through the backlog but never answers.
    let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = silent.local_addr().unwrap().port();
    std::fs::write(
        h.home.join("daemon.json"),
        json!({"pid":1,"port":port,"protocol_version":3,"instance":"busy"}).to_string(),
    )
    .unwrap();
    let e = h.error(
        &h.a,
        None,
        &["register", "--name=a", "--tool=t"],
        "DAEMON_UNREACHABLE",
    );
    assert_eq!(e["error"]["retry"], "same_key");
    assert_eq!(e["error"]["next"], "tala status");
    assert!(e["error"]["message"]
        .as_str()
        .unwrap()
        .contains("timed out"));
    assert!(!h.home.join("tala.sqlite3").exists());
    let status = h.ok(&h.a, None, &["status"]);
    assert_eq!(
        (status["running"].clone(), status["reachable"].clone()),
        (Value::Null, json!(false))
    );
    std::fs::remove_file(h.home.join("daemon.json")).unwrap();
}
#[cfg(unix)]
#[test]
fn paused_daemon_reports_transport_error_not_a_missing_recipient() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    {
        let _paused = Paused::new(&h);
        let o = h.raw(&h.a, Some(&a), &["send", "--to", &b, "hello"]);
        assert_eq!(o.status.code(), Some(1));
        let code = stderr_json(&o)["error"]["code"].clone();
        assert!(
            code == "DAEMON_UNREACHABLE" || code == "CONNECTION_ERROR",
            "{code}"
        );
    }
    assert!(!second_daemon_attempted(&h));
    h.send(&a, &b, "after resume");
}
#[cfg(unix)]
#[test]
fn blocking_send_survives_a_paused_daemon() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let waiter = h
        .command(
            &h.a,
            Some(&a),
            &["send", "--to", &b, "--wait", "--timeout=30", "question"],
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mid = wait_for_message(&h, &b);
    {
        let _paused = Paused::new(&h);
        std::thread::sleep(Duration::from_secs(3));
    }
    h.ok(&h.b, Some(&b), &["reply", &mid, "answer"]);
    let o = waiter.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["messages"][0]["parts"][0]["text"], "answer");
    assert!(!second_daemon_attempted(&h));
}
#[cfg(unix)]
#[test]
fn wait_failing_after_storage_exits_4_with_resume_command() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.b, "b");
    let waiter = h
        .command(
            &h.a,
            Some(&a),
            // Leave enough time for the in-flight RPC timeout and the retry's
            // daemon liveness probe. Otherwise the wait deadline can win while
            // the transport call is still pending, which is a timeout (3), not
            // a known transport failure (4).
            &["send", "--to", &b, "--wait", "--timeout=15", "question"],
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mid = wait_for_message(&h, &b);
    let o = {
        let _paused = Paused::new(&h);
        waiter.wait_with_output().unwrap()
    };
    assert_eq!(
        o.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let e = &stderr_json(&o)["error"];
    assert_eq!(e["code"], "WAIT_FAILED");
    assert_eq!(e["retry"], "never");
    assert_eq!(e["stored"].to_string(), mid);
    assert_eq!(
        e["next"],
        format!("tala inbox --wait --reply-to={mid} --timeout=15")
    );
    assert_eq!(
        h.ok(&h.a, Some(&a), &["pending", "--direction=outgoing"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!second_daemon_attempted(&h));
}
#[test]
fn inbox_reply_to_returns_only_the_correlated_reply() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let first = id(&h.send(&a, &b, "one"));
    let second = id(&h.send(&a, &b, "two"));
    h.ok(&h.a, Some(&b), &["reply", &second, "answer two"]);
    let none = h.raw(
        &h.a,
        Some(&a),
        &["inbox", "--wait", "--timeout=1", "--reply-to", &first],
    );
    assert_eq!(none.status.code(), Some(3));
    h.ok(&h.a, Some(&b), &["reply", &first, "answer one"]);
    let got = h.ok(&h.a, Some(&a), &["inbox", "--wait", "--reply-to", &first]);
    assert_eq!(got["messages"].as_array().unwrap().len(), 1);
    assert_eq!(got["messages"][0]["parts"][0]["text"], "answer one");
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox"])["messages"][0]["parts"][0]["text"],
        "answer two"
    );
}
#[test]
fn identical_resend_without_key_is_a_duplicate() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let c = h.register(&h.a, "c");
    let one = h.send(&a, &b, "same text");
    let two = h.send(&a, &b, "same text");
    assert_eq!(two["duplicate"], true);
    assert_eq!(two["message"]["id"], one["message"]["id"]);
    assert_eq!(h.send(&c, &b, "same text")["duplicate"], false);
    assert_eq!(h.send(&a, &b, "different text")["duplicate"], false);
}
#[test]
fn public_ids_cannot_select_an_identity() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.send(&a, &b, "for b only");
    for agent in [b.id.as_str(), "th_short"] {
        let o = h
            .command(&h.a, None, &["inbox"])
            .arg(format!("--agent={agent}"))
            .output()
            .unwrap();
        assert_eq!(stderr_json(&o)["error"]["code"], "IDENTITY_ERROR");
    }
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox"])["messages"][0]["parts"][0]["text"],
        "for b only"
    );
}

#[test]
fn saved_selector_file_selects_identity_without_exposing_handle() {
    let h = Harness::new();
    let path = h.a.join("codex selector's.selector");
    #[cfg(unix)]
    let parent_mode_before = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(&h.a).unwrap().permissions().mode()
    };
    let output = h
        .command(&h.a, None, &["register", "--name=codex", "--tool=fixture"])
        .arg(format!("--save-handle-to={}", path.display()))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let data = &value["data"];
    assert!(data.get("handle").is_none(), "{data}");
    assert!(data.get("credential").is_none(), "{data}");
    let path_text = path.to_str().unwrap();
    assert_eq!(data["selector_file"], path_text);
    let quoted_path = if cfg!(windows) {
        format!("\"{path_text}\"")
    } else {
        format!("'{}'", path_text.replace('\'', "'\\''"))
    };
    assert_eq!(data["select"], format!("--agent-file={quoted_path}"));
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.starts_with("th_") && contents.trim_end().len() == 35);
    let whoami = h.ok(
        &h.a,
        None,
        &["--agent-file", path.to_str().unwrap(), "whoami"],
    );
    assert_eq!(whoami["agent"]["id"], data["agent"]["id"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&h.a).unwrap().permissions().mode(),
            parent_mode_before
        );
    }
    let mut env_selected = h.command(&h.a, None, &["whoami"]);
    env_selected.env("TALA_AGENT_FILE", &path);
    let env_selected = env_selected.output().unwrap();
    assert!(
        env_selected.status.success(),
        "{}",
        String::from_utf8_lossy(&env_selected.stderr)
    );
    let env_selected: Value = serde_json::from_slice(&env_selected.stdout).unwrap();
    assert_eq!(env_selected["data"]["agent"]["id"], data["agent"]["id"]);
    let human_path = h.a.join("human selector's.selector");
    let human = h
        .command(&h.a, None, &["register", "--name=human", "--tool=fixture"])
        .arg(format!("--save-handle-to={}", human_path.display()))
        .arg("--human")
        .output()
        .unwrap();
    assert!(
        human.status.success(),
        "{}",
        String::from_utf8_lossy(&human.stderr)
    );
    let human = String::from_utf8(human.stdout).unwrap();
    let human_path_text = human_path.to_str().unwrap();
    let quoted_human_path = if cfg!(windows) {
        format!("\"{human_path_text}\"")
    } else {
        format!("'{}'", human_path_text.replace('\'', "'\\''"))
    };
    assert!(
        human.contains(&format!("--agent-file={quoted_human_path}")),
        "{human}"
    );
    assert!(!human.contains("th_"), "{human}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn selector_file_rejects_unsafe_contents_and_permissions() {
    let h = Harness::new();
    let agent = h.register(&h.a, "codex");
    let path = h.home.join("selector");
    std::fs::write(&path, format!("{}\nextra\n", agent.handle)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    h.error(
        &h.a,
        None,
        &["--agent-file", path.to_str().unwrap(), "whoami"],
        "IDENTITY_ERROR",
    );

    #[cfg(windows)]
    {
        // A normally inherited ACL is intentionally rejected; only Tala's
        // protected current-user + SYSTEM ACL is accepted for selector files.
        std::fs::write(&path, format!("{}\n", agent.handle)).unwrap();
        h.error(
            &h.a,
            None,
            &["--agent-file", path.to_str().unwrap(), "whoami"],
            "IDENTITY_ERROR",
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&path, format!("{}\n", agent.handle)).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        h.error(
            &h.a,
            None,
            &["--agent-file", path.to_str().unwrap(), "whoami"],
            "IDENTITY_ERROR",
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = h.home.join("selector-link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        h.error(
            &h.a,
            None,
            &["--agent-file", link.to_str().unwrap(), "whoami"],
            "IDENTITY_ERROR",
        );
    }
    let saved_path = h.home.join("valid-selector");
    let saved = h
        .command(&h.a, None, &["register", "--name=saved", "--tool=fixture"])
        .arg(format!("--save-handle-to={}", saved_path.display()))
        .output()
        .unwrap();
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let saved: Value = serde_json::from_slice(&saved.stdout).unwrap();
    assert_eq!(
        h.ok(
            &h.a,
            None,
            &["--agent-file", saved_path.to_str().unwrap(), "whoami"]
        )["agent"]["id"],
        saved["data"]["agent"]["id"]
    );
    let selector_handle = std::fs::read_to_string(&saved_path)
        .unwrap()
        .trim_end()
        .to_owned();
    std::fs::write(&saved_path, format!("{selector_handle}\r\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&saved_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert_eq!(
        h.ok(
            &h.a,
            None,
            &["--agent-file", saved_path.to_str().unwrap(), "whoami"]
        )["agent"]["id"],
        saved["data"]["agent"]["id"]
    );
    std::fs::write(&saved_path, "").unwrap();
    h.error(
        &h.a,
        None,
        &["--agent-file", saved_path.to_str().unwrap(), "whoami"],
        "IDENTITY_ERROR",
    );
    std::fs::write(&saved_path, "x".repeat(128)).unwrap();
    h.error(
        &h.a,
        None,
        &["--agent-file", saved_path.to_str().unwrap(), "whoami"],
        "IDENTITY_ERROR",
    );
    #[cfg(windows)]
    {
        let link = h.home.join("selector-reparse");
        if std::os::windows::fs::symlink_file(&saved_path, &link).is_ok() {
            h.error(
                &h.a,
                None,
                &["--agent-file", link.to_str().unwrap(), "whoami"],
                "IDENTITY_ERROR",
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn save_selector_rejects_non_utf8_path_before_registration() {
    use std::os::unix::ffi::OsStringExt;
    let h = Harness::new();
    let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b's', 0xff]));
    let output = h
        .command(
            &h.a,
            None,
            &["register", "--name=non-utf8", "--tool=fixture"],
        )
        .arg("--save-handle-to")
        .arg(path)
        .output()
        .unwrap();
    assert_eq!(stderr_json(&output)["error"]["code"], "USAGE_ERROR");
    assert!(h.ok(&h.a, None, &["agents"])["agents"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn selector_precedence_is_explicit_and_failed_save_returns_handle_once() {
    let h = Harness::new();
    let agent = h.register(&h.a, "codex");
    let selector = h.home.join("selector");
    let selector_output = h
        .command(
            &h.a,
            None,
            &["register", "--name=selector", "--tool=fixture"],
        )
        .arg(format!("--save-handle-to={}", selector.display()))
        .output()
        .unwrap();
    assert!(
        selector_output.status.success(),
        "{}",
        String::from_utf8_lossy(&selector_output.stderr)
    );

    let mut explicit_wins = h.command(&h.a, Some(&agent), &["whoami"]);
    explicit_wins.env("TALA_AGENT_FILE", &selector);
    assert!(explicit_wins.output().unwrap().status.success());

    let mut both_env = h.command(&h.a, None, &["whoami"]);
    both_env
        .env("TALA_AGENT", &agent.handle)
        .env("TALA_AGENT_FILE", &selector);
    assert_eq!(
        stderr_json(&both_env.output().unwrap())["error"]["code"],
        "IDENTITY_ERROR"
    );

    let mut empty_env = h.command(&h.a, None, &["whoami"]);
    empty_env.env("TALA_AGENT_FILE", "");
    assert_eq!(
        stderr_json(&empty_env.output().unwrap())["error"]["code"],
        "IDENTITY_ERROR"
    );
    let mut empty_handle_env = h.command(&h.a, None, &["whoami"]);
    empty_handle_env.env("TALA_AGENT", "");
    assert_eq!(
        stderr_json(&empty_handle_env.output().unwrap())["error"]["code"],
        "IDENTITY_ERROR"
    );

    let mut conflict = h.command(
        &h.a,
        None,
        &[
            "--agent=th_00000000000000000000000000000000",
            "--agent-file",
            selector.to_str().unwrap(),
            "whoami",
        ],
    );
    assert_eq!(
        stderr_json(&conflict.output().unwrap())["error"]["code"],
        "USAGE_ERROR"
    );

    let existing = h.home.join("already-exists");
    std::fs::write(&existing, "keep me").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let failed = h
        .command(&h.a, None, &["register", "--name=new", "--tool=fixture"])
        .arg(format!("--save-handle-to={}", existing.display()))
        .output()
        .unwrap();
    assert!(failed.status.success());
    let failed: Value = serde_json::from_slice(&failed.stdout).unwrap();
    let saved = &failed["data"];
    let disclosed = saved["handle"].as_str().unwrap();
    assert!(saved["selector_file_error"].is_string());
    assert!(saved.get("selector_file").is_none());
    assert_eq!(std::fs::read_to_string(existing).unwrap(), "keep me");
    let selected = h.ok(&h.a, None, &["--agent", disclosed, "whoami"]);
    assert_eq!(selected["agent"]["id"], saved["agent"]["id"]);
}
#[test]
fn reissue_issues_one_handle_for_a_legacy_identity() {
    use sha2::{Digest, Sha256};
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.send(&a, &b, "kept");
    // Recreate the protocol 2 layout: credential stored by public ID only.
    let digest: String = Sha256::digest(b.handle.as_bytes())
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    let handle_file = h.home.join("handles").join(format!("{digest}.json"));
    std::fs::create_dir_all(h.home.join("identities")).unwrap();
    std::fs::rename(
        &handle_file,
        h.home.join("identities").join(format!("{}.json", b.id)),
    )
    .unwrap();
    let issued = h.ok(&h.a, None, &["register", &format!("--reissue={}", b.id)]);
    assert_eq!(issued["agent"]["id"], b.id);
    let reissued = Ident {
        id: b.id.clone(),
        handle: issued["handle"].as_str().unwrap().into(),
    };
    assert_eq!(
        h.ok(&h.a, Some(&reissued), &["inbox"])["messages"][0]["parts"][0]["text"],
        "kept"
    );
    h.error(
        &h.a,
        None,
        &["register", &format!("--reissue={}", b.id)],
        "IDENTITY_ERROR",
    );
    h.ok(&h.a, Some(&reissued), &["whoami"]);
}
#[test]
fn handles_never_appear_in_shared_output() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let sent = h.send(&a, &b, "parser");
    h.ok(&h.a, Some(&a), &["post", "parser update"]);
    let outputs = [
        h.ok(&h.a, None, &["agents"]),
        h.ok(&h.a, Some(&b), &["history", &thread(&sent)]),
        h.ok(&h.a, Some(&b), &["search", "parser"]),
        h.ok(&h.a, Some(&b), &["board"]),
        h.ok(&h.a, Some(&b), &["inbox"]),
        h.ok(&h.a, Some(&b), &["pending"]),
        h.ok(&h.a, Some(&b), &[]),
    ];
    for output in outputs {
        let text = output.to_string();
        assert!(
            !text.contains(&a.handle) && !text.contains(&b.handle),
            "{text}"
        );
    }
}
#[test]
fn json_is_the_default_and_human_renders_text() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.send(&a, &b, "rendered");
    let plain = |args: &[&str]| {
        let mut c = Command::new(bin());
        c.env("TALA_HOME", &h.home)
            .env("TALA_AGENT", &b.handle)
            .env_remove("TALA_AGENT_FILE")
            .current_dir(&h.a)
            .args(args);
        c.output().unwrap()
    };
    let o = plain(&["inbox", "--peek"]);
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["ok"], true);
    let o = plain(&["inbox", "--peek", "--human"]);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("rendered") && text.contains("ago"), "{text}");
    assert!(text.contains("Attention: 1 unread"), "{text}");
    let o = plain(&["--human"]);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(
        text.contains("1 requests to you") && text.contains("Next: tala inbox"),
        "{text}"
    );
    let o = plain(&["send", "--to=missing", "x", "--human"]);
    assert!(String::from_utf8(o.stderr)
        .unwrap()
        .contains("Next: tala agents"));
}
fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success());
}
#[test]
fn sibling_worktrees_share_one_board() {
    let h = Harness::new();
    git(&h.a, &["init", "-q"]);
    git(&h.a, &["commit", "-q", "--allow-empty", "-m", "init"]);
    let wt = h.temp.path().join("wt");
    git(
        &h.a,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "side"],
    );
    git(&h.b, &["init", "-q"]);
    let main = h.register(&h.a, "main");
    let side = h.register(&wt, "side");
    let other = h.register(&h.b, "other");
    let root = std::fs::canonicalize(&h.a).unwrap().display().to_string();
    let who = h.ok(&wt, Some(&side), &["whoami"]);
    assert_eq!(who["agent"]["context"]["project"], root);
    assert_eq!(
        who["agent"]["context"]["worktree"],
        std::fs::canonicalize(&wt).unwrap().display().to_string()
    );
    h.ok(&h.a, Some(&main), &["post", "changing parser"]);
    let board = h.ok(&wt, Some(&side), &["board"]);
    assert_eq!(board["messages"][0]["parts"][0]["text"], "changing parser");
    assert_eq!(board["messages"][0]["context"]["worktree"], root);
    assert_eq!(h.ok(&h.b, Some(&other), &["board"])["messages"], json!([]));
    // A message from the linked worktree records that worktree and branch,
    // while staying on the shared project board.
    let posted = h.ok(&wt, Some(&side), &["post", "changing report"]);
    let context = &posted["message"]["context"];
    assert_eq!(context["project"], root);
    assert_eq!(
        context["worktree"],
        std::fs::canonicalize(&wt).unwrap().display().to_string()
    );
    assert_eq!(context["branch"], "side");
    assert_eq!(
        h.ok(&h.a, Some(&main), &["board"])["messages"][1]["parts"][0]["text"],
        "changing report"
    );
}

#[test]
fn outgoing_message_uses_invocation_linked_worktree_branch_revision_and_directory() {
    let h = Harness::new();
    git(&h.a, &["init", "-q"]);
    git(&h.a, &["branch", "-M", "main"]);
    git(
        &h.a,
        &["commit", "-q", "--allow-empty", "-m", "registered revision"],
    );
    let wt = h.temp.path().join("invocation-worktree");
    git(
        &h.a,
        &[
            "worktree",
            "add",
            "-q",
            wt.to_str().unwrap(),
            "-b",
            "feature",
        ],
    );
    git(
        &wt,
        &["commit", "-q", "--allow-empty", "-m", "invocation revision"],
    );
    let nested = wt.join("src").join("nested");
    std::fs::create_dir_all(&nested).unwrap();

    let sender = h.register(&h.a, "sender");
    let reader = h.register(&h.a, "reader");
    let posted = h.ok(
        &nested,
        Some(&sender),
        &["post", "current worktree context"],
    );
    let message_context = &posted["message"]["context"];
    let project = h.a.canonicalize().unwrap().display().to_string();
    let canonical_wt = wt.canonicalize().unwrap().display().to_string();
    let canonical_nested = nested.canonicalize().unwrap().display().to_string();
    let expected_revision = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&wt)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();

    assert_eq!(message_context["project"], project);
    assert_eq!(message_context["working_directory"], canonical_nested);
    assert_eq!(message_context["worktree"], canonical_wt);
    assert_eq!(message_context["branch"], "feature");
    assert_eq!(message_context["revision"], expected_revision);
    assert_eq!(
        h.ok(&h.a, Some(&reader), &["board"])["messages"][0]["context"]["project"],
        project
    );
    assert_eq!(
        h.ok(&h.a, Some(&sender), &["whoami"])["agent"]["context"]["branch"],
        "main"
    );
}
#[test]
fn bare_tala_summarises_work_without_acknowledging_it() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let empty = h.ok(&h.a, Some(&b), &[]);
    assert_eq!(empty["incoming_requests"], json!([]));
    assert_eq!(empty["unread"]["count"], 0);
    assert!(empty["next"].is_null());
    h.error(&h.a, None, &[], "IDENTITY_REQUIRED");
    let request = id(&h.send(&a, &b, "review"));
    h.ok(&h.a, Some(&a), &["send", "--to", &b, "fyi"]);
    let summary = h.ok(&h.a, Some(&b), &[]);
    assert_eq!(summary["incoming_requests"][0]["id"].to_string(), request);
    assert_eq!(
        summary["incoming_requests"][0]["next"],
        format!("tala reply {request} '<answer>'")
    );
    assert_eq!(summary["unread"]["count"], 2);
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox", "--peek"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let waiting = h.ok(&h.a, Some(&a), &[]);
    assert_eq!(
        waiting["outgoing_requests"][0]["next"],
        format!("tala inbox --wait --reply-to={request}")
    );
    // An unregistered sender makes the request stale.
    h.ok(&h.a, Some(&a), &["unregister"]);
    let summary = h.ok(&h.a, Some(&b), &[]);
    assert_eq!(summary["incoming_requests"], json!([]));
    assert_eq!(summary["stale"]["incoming"], 1);
    assert_eq!(
        h.ok(&h.a, Some(&b), &["pending", "--stale"])["messages"][0]["id"].to_string(),
        request
    );
}
#[test]
fn every_response_carries_ambient_attention() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let envelope = |agent: &Ident, args: &[&str]| -> Value {
        let o = h.raw(&h.a, Some(agent), args);
        assert!(o.status.success());
        serde_json::from_slice(&o.stdout).unwrap()
    };
    assert!(envelope(&b, &["post", "nothing waiting"])
        .get("attention")
        .is_none());
    assert!(envelope(&b, &["agents"]).get("attention").is_none());
    let request = id(&h.send(&a, &b, "review this"));
    let posted = envelope(&b, &["post", "working"]);
    assert_eq!(posted["data"]["message"]["parts"][0]["text"], "working");
    let attention = &posted["attention"];
    assert_eq!(attention["requests"][0]["id"].to_string(), request);
    assert_eq!(attention["requests"][0]["from"], a.id);
    assert!(!attention.to_string().contains("review this"));
    assert_eq!(envelope(&b, &["agents"])["attention"]["open_requests"], 1);
    let consumed = envelope(&b, &["inbox"]);
    assert_eq!(consumed["attention"]["unread"], 0);
    assert_eq!(consumed["attention"]["open_requests"], 1);
    assert_eq!(
        h.ok(&h.a, Some(&b), &["inbox", "--peek"])["messages"],
        json!([])
    );
}
#[test]
fn board_reply_reaches_the_posts_author() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    let plan = id(&h.ok(&h.a, Some(&a), &["post", "changing parser"]));
    let waiter = h
        .command(&h.a, Some(&a), &["inbox", "--wait", "--timeout=10"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    h.ok(&h.a, Some(&b), &["reply", &plan, "no overlap"]);
    let o = waiter.wait_with_output().unwrap();
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["messages"][0]["parts"][0]["text"], "no overlap");
    assert_eq!(v["data"]["messages"][0]["reply_to"].to_string(), plan);
    // The reply stays on the board, and reading the board acknowledges nothing.
    let second = id(&h.ok(&h.a, Some(&b), &["reply", &plan, "one more"]));
    assert_eq!(
        h.ok(&h.a, Some(&a), &["board"])["messages"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        h.ok(&h.a, Some(&a), &["inbox"])["messages"][0]["id"].to_string(),
        second
    );
    assert_eq!(h.ok(&h.a, Some(&b), &["inbox"])["messages"], json!([]));
}
#[test]
fn board_wait_returns_only_new_messages() {
    let h = Harness::new();
    let a = h.register(&h.a, "a");
    let b = h.register(&h.a, "b");
    h.ok(&h.a, Some(&a), &["post", "existing"]);
    let o = h.raw(&h.a, Some(&b), &["board", "--wait", "--timeout=1"]);
    assert_eq!(o.status.code(), Some(3));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["messages"], json!([]));
    let waiter = h
        .command(&h.a, Some(&b), &["board", "--wait", "--timeout=10"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    h.ok(&h.a, Some(&a), &["post", "new plan"]);
    let o = waiter.wait_with_output().unwrap();
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["messages"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"]["messages"][0]["parts"][0]["text"], "new plan");
    // An explicit cursor returns anything newer at once.
    let all = h.ok(&h.a, Some(&b), &["board", "--wait", "--after=1"]);
    assert_eq!(all["messages"][0]["parts"][0]["text"], "new plan");
    assert_eq!(h.ok(&h.a, Some(&b), &["inbox"])["messages"], json!([]));
}
