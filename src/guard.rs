//! Shared-board guardrail: never mutate a teammate's task by accident.
//!
//! Team E has 17 members on one board; an agent loop that edits the wrong card
//! disturbs real people, so mutation of a task owned by someone else requires
//! an explicit `--force`.

use crate::api::models::Task;

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error(
        "Task {task} is assigned to user `{responsible_user_id}`, not you (`{my_user_id}`). \
         Re-run with --force if you really mean to change a teammate's task."
    )]
    NotResponsible {
        task: String,
        responsible_user_id: String,
        my_user_id: String,
    },
    #[error(
        "Task {task} has no responsible user, so it is not yours to change. \
         Use `kf task grab {task}` to take it, or re-run with --force."
    )]
    Unassigned { task: String },
}

/// Allow the mutation when `--force` was given, or the task is ours.
pub fn ensure_can_mutate(task: &Task, my_user_id: &str, force: bool) -> Result<(), GuardError> {
    if force {
        return Ok(());
    }
    match task.responsible_user_id.as_deref() {
        Some(owner) if owner == my_user_id => Ok(()),
        Some(owner) => Err(GuardError::NotResponsible {
            task: task.reference(),
            responsible_user_id: owner.to_string(),
            my_user_id: my_user_id.to_string(),
        }),
        None => Err(GuardError::Unassigned {
            task: task.reference(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(responsible_user_id: Option<&str>) -> Task {
        let mut task: Task = serde_json::from_str(
            r#"{"_id":"T3s6UGyzY","name":"Write report","columnId":"C9LIn5sEEpqT"}"#,
        )
        .expect("fixture parses");
        task.responsible_user_id = responsible_user_id.map(str::to_string);
        task
    }

    #[test]
    fn own_task_may_be_mutated() {
        assert!(ensure_can_mutate(&task(Some("UME")), "UME", false).is_ok());
    }

    #[test]
    fn teammates_task_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&task(Some("UOTHER")), "UME", false),
            Err(GuardError::NotResponsible { .. })
        ));
        assert!(ensure_can_mutate(&task(Some("UOTHER")), "UME", true).is_ok());
    }

    #[test]
    fn unassigned_task_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&task(None), "UME", false),
            Err(GuardError::Unassigned { .. })
        ));
        assert!(ensure_can_mutate(&task(None), "UME", true).is_ok());
    }
}
