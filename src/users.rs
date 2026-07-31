//! Turning user IDs into names for human-readable output.
//!
//! Task, comment and attachment payloads carry only user IDs. `GET /users` maps
//! them to names, but it costs a request out of the board's 1000/hour budget and
//! only human output needs it — so the fetch happens lazily, at most once per
//! invocation, and a failed fetch degrades to printing the raw IDs.

use std::collections::HashMap;

use crate::api::models::User;
use crate::api::Client;

pub struct UserNames<'a> {
    client: &'a Client,
    /// The token's own user, rendered as `me`. `None` where that would be
    /// misleading, such as a comment listing that shows real authors.
    my_user_id: Option<&'a str>,
    names: Option<HashMap<String, String>>,
}

impl<'a> UserNames<'a> {
    /// Render the token's own user ID as `me`.
    pub fn new(client: &'a Client, my_user_id: &'a str) -> Self {
        Self {
            client,
            my_user_id: Some(my_user_id),
            names: None,
        }
    }

    /// Render every user, including the token's own, by name.
    pub fn without_me(client: &'a Client) -> Self {
        Self {
            client,
            my_user_id: None,
            names: None,
        }
    }

    /// The user's name, or the raw ID when the board does not know it.
    pub fn display(&mut self, user_id: &str) -> String {
        if self.my_user_id == Some(user_id) {
            return "me".to_string();
        }
        self.names()
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| user_id.to_string())
    }

    fn names(&mut self) -> &HashMap<String, String> {
        self.names.get_or_insert_with(|| {
            // A failed lookup is not worth failing the whole read over; the raw
            // user IDs are still printed.
            self.client
                .get_json::<Vec<User>>("users", &[])
                .map(|users| {
                    users
                        .into_iter()
                        .map(|user| (user.id, user.full_name))
                        .collect()
                })
                .unwrap_or_default()
        })
    }
}
