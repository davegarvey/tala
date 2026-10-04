use crate::{cli, models::*, store};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
const GUIDE: &str = include_str!("../docs/agent-guide.md");
struct Planned {
    path: PathBuf,
    old: Option<String>,
    action: &'static str,
    stage: Option<PathBuf>,
}
pub fn init(check: bool, refresh: bool, dry_run: bool) -> Result<Value> {
    let root = cli::context(None)?.project;
    let root = Path::new(&root);
    let content = format!("---\nname: tala\ndescription: Local messaging between distinct coding agents.\ntala_cli_version: {}\n---\n\n{}",env!("CARGO_PKG_VERSION"),GUIDE);
    let mut paths = vec![root.join(".tala/AGENTS.md")];
    if root.join(".opencode").is_dir() {
        paths.push(root.join(".opencode/skills/tala/SKILL.md"));
        paths.push(root.join(".opencode/commands/tala.md"));
    }
    // Inspect all destinations before modifying any documents.
    let mut plan = Vec::new();
    for path in paths {
        let old = match std::fs::read_to_string(&path) {
            Ok(s) => Some(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let action = match old.as_deref() {
            Some(s) if s == content => "unchanged",
            None => "create",
            Some(_) if refresh => "replace",
            Some(_) => "skipped",
        };
        plan.push(Planned {
            path,
            old,
            action,
            stage: None,
        });
    }
    if !check && !dry_run {
        let staged = (|| -> Result<()> {
            for item in &mut plan {
                if ["create", "replace"].contains(&item.action) {
                    std::fs::create_dir_all(item.path.parent().expect("integration parent"))?;
                    let stage = item
                        .path
                        .with_extension(format!("{}.stage", uuid::Uuid::new_v4()));
                    store::private_write(&stage, content.as_bytes())?;
                    item.stage = Some(stage);
                }
            }
            Ok(())
        })();
        if let Err(e) = staged {
            for item in &plan {
                if let Some(stage) = &item.stage {
                    let _ = std::fs::remove_file(stage);
                }
            }
            return Err(e);
        }
        let mut applied: Vec<usize> = Vec::new();
        for (index, item) in plan.iter().enumerate() {
            if let Some(stage) = &item.stage {
                if let Err(error) = std::fs::rename(stage, &item.path) {
                    let mut rollback_errors = Vec::new();
                    for &index in applied.iter().rev() {
                        let original = &plan[index];
                        let result = if let Some(old) = &original.old {
                            store::private_write(&original.path, old.as_bytes())
                        } else {
                            std::fs::remove_file(&original.path)
                        };
                        if let Err(e) = result {
                            rollback_errors.push(e.to_string());
                        }
                    }
                    for item in &plan {
                        if let Some(stage) = &item.stage {
                            let _ = std::fs::remove_file(stage);
                        }
                    }
                    return Err(Failure::new(
                        "INTEGRATION_WRITE_FAILED",
                        format!("{error}; rollback errors: {}", rollback_errors.join(", ")),
                        "Inspect the reported integration destinations before retrying.",
                    ));
                }
                applied.push(index);
            }
        }
    }
    let actions:Vec<_>=plan.iter().map(|item|json!({"path":item.path,"action":item.action,"hint":if item.action=="skipped"{"Use init --refresh to replace customized or stale instructions."}else{""}})).collect();
    Ok(
        json!({"project":root,"version":env!("CARGO_PKG_VERSION"),"check":check,"dry_run":dry_run,"actions":actions}),
    )
}
