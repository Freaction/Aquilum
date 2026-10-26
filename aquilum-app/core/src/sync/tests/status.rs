use crate::sync::state::State;
use crate::sync::{Outcome, RemoteError, SyncError, SyncStatus};

fn settled(passed: Result<Vec<(&str, RemoteError)>, RemoteError>) -> SyncStatus {
    let mut state = State::default();
    let passed = passed.map(|failed| Outcome { failed: failed.into_iter().map(|(path, error)| (path.to_owned(), error.into())).collect(), ..Outcome::default() });
    state.settle(passed.map_err(SyncError::from));
    state.status(true)
}

fn offline() -> RemoteError {
    RemoteError::Offline("Dns Failed".into())
}

#[test]
fn a_lost_network_is_no_error() {
    assert_eq!(settled(Err(offline())), SyncStatus::Offline);
    assert_eq!(settled(Ok(vec![("Один.md", offline()), ("Два.md", offline())])), SyncStatus::Offline);
}

#[test]
fn file_errors_are_summed_up_with_the_first_real_one() {
    let failed = vec![("Один.md", offline()), ("Два.md", RemoteError::Failed("Dropbox 400: плохо\nподробности".into())), ("Три.md", RemoteError::Stale)];

    assert_eq!(settled(Ok(failed)), SyncStatus::Unsynced { files: 3, first: "Два.md: Dropbox 400: плохо".into() });
    assert_eq!(settled(Err(RemoteError::Failed("папка пропала".into()))), SyncStatus::Failed("папка пропала".into()));
}
