//! Shared-board guardrail: never mutate a teammate's issue by accident.
//!
//! Team E has 17 members on one board; an agent loop that edits the wrong issue
//! disturbs real people, so mutation of an issue owned by someone else requires
//! an explicit `--force`.

use crate::api::models::Issue;

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error(
        "Issue {issue} is assigned to user `{responsible_user_id}`, not you (`{my_user_id}`), \
         and you are not a collaborator on it. \
         Re-run with --force if you really mean to change a teammate's issue."
    )]
    NotMine {
        issue: String,
        responsible_user_id: String,
        my_user_id: String,
    },
    #[error(
        "Issue {issue} has nobody on it, so it is not yours to change. \
         Use `kf issue grab {issue}` to take it, or re-run with --force."
    )]
    Unassigned { issue: String },
    #[error(
        "Issue {issue} is assigned to user `{responsible_user_id}`, not you (`{my_user_id}`). \
         Collaborating on an issue does not make it yours to take over — \
         re-run with --force to reassign a teammate's issue to yourself."
    )]
    NotYoursToTakeOver {
        issue: String,
        responsible_user_id: String,
        my_user_id: String,
    },
}

/// An issue counts as yours when you are its responsible user **or** one of its
/// collaborators: KanbanFlow's board draws an avatar for either, so a
/// collaborator-only issue reads as assigned to you — both for `--mine` and for
/// the mutation guardrail.
pub fn is_mine(issue: &Issue, my_user_id: &str) -> bool {
    issue.responsible_user_id.as_deref() == Some(my_user_id)
        || issue
            .collaborators
            .iter()
            .any(|collaborator| collaborator.user_id == my_user_id)
}

/// Allow the mutation when `--force` was given, or the issue is ours.
pub fn ensure_can_mutate(issue: &Issue, my_user_id: &str, force: bool) -> Result<(), GuardError> {
    if force || is_mine(issue, my_user_id) {
        return Ok(());
    }
    match issue.responsible_user_id.as_deref() {
        Some(owner) => Err(GuardError::NotMine {
            issue: issue.reference(),
            responsible_user_id: owner.to_string(),
            my_user_id: my_user_id.to_string(),
        }),
        None => Err(GuardError::Unassigned {
            issue: issue.reference(),
        }),
    }
}

/// Taking an issue over — `kf issue grab` — reassigns the responsible user, so it is
/// judged on that alone. Collaborating on a teammate's issue must not silently let
/// us pull their name off it; an unassigned issue is still fair game.
pub fn ensure_can_take_over(
    issue: &Issue,
    my_user_id: &str,
    force: bool,
) -> Result<(), GuardError> {
    if force {
        return Ok(());
    }
    match issue.responsible_user_id.as_deref() {
        Some(owner) if owner != my_user_id => Err(GuardError::NotYoursToTakeOver {
            issue: issue.reference(),
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

    fn issue(responsible_user_id: Option<&str>, collaborators: &[&str]) -> Issue {
        let mut issue: Issue = serde_json::from_str(
            r#"{"_id":"T3s6UGyzY","name":"Write report","columnId":"C9LIn5sEEpqT"}"#,
        )
        .expect("fixture parses");
        issue.responsible_user_id = responsible_user_id.map(str::to_string);
        issue.collaborators = collaborators
            .iter()
            .map(|user_id| Collaborator {
                user_id: (*user_id).to_string(),
            })
            .collect();
        issue
    }

    #[test]
    fn own_issue_may_be_mutated() {
        assert!(ensure_can_mutate(&issue(Some("UME"), &[]), "UME", false).is_ok());
    }

    /// The team assigns work by adding collaborators, so a `+me` issue is as much
    /// mine as one I am responsible for.
    #[test]
    fn collaborating_is_enough_to_mutate_even_when_someone_else_is_responsible() {
        assert!(ensure_can_mutate(&issue(None, &["UME"]), "UME", false).is_ok());
        assert!(ensure_can_mutate(&issue(Some("UOTHER"), &["UME"]), "UME", false).is_ok());
    }

    #[test]
    fn teammates_issue_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&issue(Some("UOTHER"), &["UTHIRD"]), "UME", false),
            Err(GuardError::NotMine { .. })
        ));
        assert!(ensure_can_mutate(&issue(Some("UOTHER"), &[]), "UME", true).is_ok());
    }

    #[test]
    fn unassigned_issue_is_refused_unless_forced() {
        assert!(matches!(
            ensure_can_mutate(&issue(None, &[]), "UME", false),
            Err(GuardError::Unassigned { .. })
        ));
        assert!(ensure_can_mutate(&issue(None, &[]), "UME", true).is_ok());
    }

    #[test]
    fn taking_over_looks_only_at_the_responsible_user() {
        assert!(ensure_can_take_over(&issue(None, &[]), "UME", false).is_ok());
        assert!(ensure_can_take_over(&issue(Some("UME"), &[]), "UME", false).is_ok());
        assert!(matches!(
            ensure_can_take_over(&issue(Some("UOTHER"), &["UME"]), "UME", false),
            Err(GuardError::NotYoursToTakeOver { .. })
        ));
        assert!(ensure_can_take_over(&issue(Some("UOTHER"), &["UME"]), "UME", true).is_ok());
    }
}
