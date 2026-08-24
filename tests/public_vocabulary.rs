use std::path::{Path, PathBuf};

#[test]
fn standalone_legacy_vocabulary_is_confined_to_upstream_wire_contracts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for relative in [
        "src",
        "docs",
        "skills",
        ".claude-plugin",
        ".mpx",
        "README.md",
    ] {
        visit(&root.join(relative), &root, &mut violations);
    }
    assert!(
        violations.is_empty(),
        "standalone legacy vocabulary remains outside wire/API documentation allowances:\n{}",
        violations.join("\n")
    );
}

fn visit(path: &Path, root: &Path, violations: &mut Vec<String>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("read source directory") {
            visit(&entry.expect("directory entry").path(), root, violations);
        }
        return;
    }
    let relative = path.strip_prefix(root).expect("repository file");
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.starts_with("docs/api/")
        || normalized == "src/api/models.rs"
        || normalized == "src/api/client.rs"
        || normalized == "tests/public_vocabulary.rs"
    {
        return;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for (index, line) in text.lines().enumerate() {
        if line.trim() == "Say \"issue\", never \"task\" or \"card\"." {
            continue;
        }
        let mut scrubbed = line.to_string();
        if normalized.starts_with("src/") {
            for wire_token in [
                "docs/api/get-tasks.md",
                "docs/api/create-task.md",
                "/tasks",
                "tasks/",
                "\"tasks\"",
                ".tasks",
                "TaskGroup",
                "CreateTask",
                "UpdateTask",
                "TaskNumber",
                "TaskDate",
                "task_id",
                "task_comment_id",
                "task_attachment_id",
                "sub_tasks",
                "startTaskId",
                "tasksLimited",
                "nextTaskId",
                "`Task`",
                "subTasks",
                "[\"kf\", \"task\", \"list\"]",
            ] {
                scrubbed = scrubbed.replace(wire_token, "");
            }
        }
        if words(&scrubbed).any(is_forbidden_word) {
            violations.push(format!("{}:{}: {}", normalized, index + 1, line.trim()));
        }
    }
    for component in relative.components() {
        let name = component.as_os_str().to_string_lossy();
        if words(&name).any(is_forbidden_word) {
            violations.push(format!("legacy vocabulary in path: {normalized}"));
            break;
        }
    }
}

fn is_forbidden_word(word: &str) -> bool {
    ["task", "tasks", "card", "cards"]
        .iter()
        .any(|forbidden| word.eq_ignore_ascii_case(forbidden))
}

fn words(line: &str) -> impl Iterator<Item = &str> {
    line.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
}
