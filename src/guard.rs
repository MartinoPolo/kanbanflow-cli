//! Shared-board guardrail: never mutate a teammate's task by accident.
//!
//! Team E has 17 members on one board; an agent loop that edits the wrong card
//! disturbs real people, so mutation of a task owned by someone else requires
//! an explicit `--force`.

use crate::api::models::Task;

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error(
        "Task {task} is assigned to user `{responsible_user_id}`, not you (`{my_user_id}`), \
         and you are not a collaborator on it. \
         Re-run with --force if you really mean to change a teammate's task."
    )]
    NotMine {
        task: String,
        responsible_user_id: String,
        my_user_id: String,
    },
    #[error(
        "Task {task} has nobody on it, so it is not yours to change. \
         Use `kf task grab {task}` to take it, or re-run with --force."
    )]
    Unassigned { task: String },
    #[error(
        "Task {task} is assigned to user `{responsible_user_id}`, not you (`{my_user_id}`). \
         Collaborating on a task does not make it yours to take over — \
         re-run with --force to reassign a teammate's task to yourself."
    )]
    NotYoursToTakeOver {
        task: String,
        responsible_user_id: String,
        my_user_id: String,
    },
}

/// A task counts as yours when you are its responsible user **or** one of its
/// collaborators: KanbanFlow's board draws an avatar for either, so a
/// collaborator-only task reads as assigned to you — both for `--mine` and for
/// the mutation guardrail.
pub fn is_mine(task: &Task, my_user_id: &str) -> bool {
    task.responsible_user_id.as_deref() == Some(my_user_id)
        || task
            .collaborators
            .iter()
            .any(|collaborator| collaborator.user_id == my_user_id)
}

/// Allow the mutation when `--force` was given, or the task is ours.
pub fn ensure_can_mutate(task: &Task, my_user_id: &str, force: bool) -> Result<(), GuardError> {
    if force || is_mine(task, my_user_id) {
        return Ok(());
    }
    match task.responsible_user_id.as_deref() {
        Some(owner) => Err(GuardError::NotMine {
            task: task.reference(),
            responsible_user_id: owner.to_string(),
            my_user_id: my_user_id.to_string(),
        }),
        None => Err(GuardError::Unassigned {
            task: task.reference(),
        }),
    }
}

/// Taking a task over — `kf task grab` — reassigns the responsible user, so it is
/// judged on that alone. Collaborating on a teammate's task must not silently let
/// us pull their name off it; an unassigned task is still fair game.
pub fn ensure_can_take_over(task: &Task, my_user_id: &str, force: bool) -> Result<(), GuardError> {
    if force {
        return Ok(());
    }
    match task.responsible_user_id.as_deref() {
        Some(owner) if owner != my_user_id => Err(GuardError::NotYoursToTakeOver {
            task: task.reference(),
            responsible_user_id: owner.to_string(),
            my_user_id: my_user_id.to_string(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::Collaborator;

    fn task(responsible_user_id: Option<&str>, collaborators: &[&str]) -> Task {
        let mut task: Task = serde_json::from_str(
            r#"{"_id":"T3s6UGyzY","name":"Write report","columnId":"C9LIn5sEEpqT"}"#,
        )
        .expect("fixture parses");
        task.responsible_user_id = responsible_user_id.map(str::to_string);
        task.collaborators = collaborators
            .iter()
            .map(|user_id| Collaborator {
                user_id: (*user_id).to_string(),
            })
            .collect();
        task
    }

    #[test]
    fn own_task_may_be_mutated() {
        assert!(ensure_can_mutate(&task(Some("UME"), &[]), "UME", false).is_ok());
    }

    /// The team assigns work by adding collaborators, so a `+me` task is as much
    /// mine as one I am responsible for.
    #[test]
    fn collaborating_is_enough_to_mutate_even_when_someone_else_is_responsible() {
        assert!(ensure_can_mutate(&task(None, &["UME"]), "UME", false).is_ok());
        assert!(ensure_can_mutate(&task(Some("UOTHER"), &["UME"]), "UME", false).is_ok());
    }

    #[test]
    fn teammates_task_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&task(Some("UOTHER"), &["UTHIRD"]), "UME", false),
            Err(GuardError::NotMine { .. })
        ));
        assert!(ensure_can_mutate(&task(Some("UOTHER"), &[]), "UME", true).is_ok());
    }

    #[test]
    fn unassigned_task_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&task(None, &[]), "UME", false),
            Err(GuardError::Unassigned { .. })
        ));
        assert!(ensure_can_mutate(&task(None, &[]), "UME", true).is_ok());
    }

    #[test]
    fn taking_over_looks_only_at_the_responsible_user() {
        assert!(ensure_can_take_over(&task(None, &[]), "UME", false).is_ok());
        assert!(ensure_can_take_over(&task(Some("UME"), &[]), "UME", false).is_ok());
        assert!(matches!(
            ensure_can_take_over(&task(Some("UOTHER"), &["UME"]), "UME", false),
            Err(GuardError::NotYoursToTakeOver { .. })
        ));
        assert!(ensure_can_take_over(&task(Some("UOTHER"), &["UME"]), "UME", true).is_ok());
    }
}
