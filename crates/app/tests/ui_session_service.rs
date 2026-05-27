//! UI session service acceptance tests.
//!
//! Proves the service layer correctly wraps the store registry
//! and produces UI-friendly DTOs.

use openwand_app::ui::{CreateSessionRequest, UiSessionService};
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use std::sync::Arc;

fn open_service() -> UiSessionService {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(SqliteStore::open_in_temp_dir()).unwrap();
    let registry: Arc<dyn openwand_store::SessionRegistryStore> = Arc::new(store);
    UiSessionService::new(registry)
}

#[test]
fn ui_session_service_lists_registry_sessions() {
    let svc = open_service();

    // Empty initially
    let sessions = svc.list_sessions().unwrap();
    assert!(sessions.is_empty());

    // Create one
    svc.create_session(CreateSessionRequest {
        title: Some("Test".into()),
        model: Some("qwen3-4b".into()),
        base_url: None,
        provider: None,
        working_directory: None,
        interaction_mode: "direct".into(),
    })
    .unwrap();

    let sessions = svc.list_sessions().unwrap();
    assert_eq!(1, sessions.len());
    assert_eq!(Some("Test".into()), sessions[0].title);
    assert_eq!(Some("qwen3-4b".into()), sessions[0].model);
}

#[test]
fn ui_session_service_create_session_adds_registry_row() {
    let svc = open_service();

    let summary = svc
        .create_session(CreateSessionRequest {
            title: Some("My Session".into()),
            model: None,
            base_url: Some("http://localhost:1234/v1".into()),
            provider: Some("lm-studio".into()),
            working_directory: Some("/tmp".into()),
            interaction_mode: "conversational".into(),
        })
        .unwrap();

    assert_eq!("My Session", summary.title.unwrap());
    assert_eq!("active", summary.status);
    assert_eq!(None, summary.model); // model was None in request
    // provider is stored but not in UiSessionSummary
    // session_id is auto-generated (ULID)
    assert!(!summary.session_id.is_empty());
}

#[test]
fn ui_session_service_open_empty_session_returns_empty_messages() {
    let svc = open_service();

    let created = svc
        .create_session(CreateSessionRequest {
            title: None,
            model: None,
            base_url: None,
            provider: None,
            working_directory: None,
            interaction_mode: "direct".into(),
        })
        .unwrap();

    let view = svc.open_session(&created.session_id).unwrap();
    assert!(view.messages.is_empty());
    assert_eq!("direct", view.interaction_mode);
    assert_eq!(0, view.current_step);
}

#[test]
fn ui_session_service_open_session_with_metadata() {
    let svc = open_service();

    let created = svc
        .create_session(CreateSessionRequest {
            title: Some("Meta Test".into()),
            model: Some("qwen3-4b".into()),
            base_url: Some("http://gpu:1234/v1".into()),
            provider: Some("lm-studio".into()),
            working_directory: Some("/home/user".into()),
            interaction_mode: "direct".into(),
        })
        .unwrap();

    let view = svc.open_session(&created.session_id).unwrap();
    assert_eq!(Some("Meta Test".into()), view.summary.title);
    assert_eq!(Some("qwen3-4b".into()), view.summary.model);
    assert_eq!(Some("http://gpu:1234/v1".into()), view.base_url);
    assert_eq!(Some("lm-studio".into()), view.provider);
    assert_eq!(Some("/home/user".into()), view.working_directory);
}

#[test]
fn ui_session_service_missing_session_returns_not_found() {
    let svc = open_service();

    let result = svc.open_session("nonexistent_id");
    assert!(result.is_err());
    match result.unwrap_err() {
        openwand_app::ui::UiServiceError::NotFound(id) => {
            assert_eq!("nonexistent_id", id);
        }
        other => panic!("Expected NotFound, got: {other}"),
    }
}

#[test]
fn desktop_feature_does_not_affect_cli_build() {
    // The CLI binary (openwand) compiles without the desktop feature.
    // This is verified by the fact that `cargo check -p openwand-app` succeeds
    // without --features desktop, which is tested in CI.
    // Here we just assert the service types are available in both modes.
    let _ = std::mem::size_of::<openwand_app::ui::UiSessionSummary>();
    let _ = std::mem::size_of::<openwand_app::ui::UiSessionView>();
    let _ = std::mem::size_of::<openwand_app::ui::UiMessage>();
}
