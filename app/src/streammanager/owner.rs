//! V109-C04 Whose channel a dashboard panel acts on. The stream manager sends no account:
//! the manager's (`gql_call`). The creator dashboard sends the account picked in its header:
//! that account's own channel, same sign-in and mock rules (`gql_call_as`).

use tauri::AppHandle;

use super::gql_call::{GqlCall, gql_call, gql_call_as};

const NO_ACCOUNT: &str = "That account isn’t signed in here.";

/// The GQL call on `account`'s own channel, or the manager's when `None`.
pub fn own_call(app: &AppHandle, account: Option<&str>) -> Result<GqlCall, String> {
    let Some(id) = account else { return gql_call(app) };
    let accounts = crate::auth::accounts(app);
    let acc = accounts.get(id).ok_or(NO_ACCOUNT)?;
    gql_call_as(app, id, acc.user_id.clone(), acc.login.clone())
}
