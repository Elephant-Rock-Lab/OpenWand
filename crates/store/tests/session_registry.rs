//! Session registry acceptance tests.
//!
//! Proves the registry seam:
//! - create/list/get/update/archive
//! - registry is navigation metadata, not authority
//! - trace replay works independently of registry

use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::{NewSessionRecord, SessionListFilter, SessionRegistryStore, SessionRegistryUpdate};
use openwand_trace::store::TraceStore;

fn open_test_store() -> SqliteStore {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(SqliteStore::open_in_temp_dir()).unwrap()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

#[test]
fn session_registry_create_persists_row() {
    let store = open_test_store();

    let record = store.create_session(NewSessionRecord {
        session_id: "sess_001".into(),
        title: Some("Test Session".into()),
        provider: Some("lm-studio".into()),
        model: Some("qwen3-4b".into()),
        base_url: Some("http://localhost:1234/v1".into()),
        working_directory: Some("/tmp".into()),
        interaction_mode: "direct".into(),
    }).unwrap();

    assert_eq!("sess_001", record.session_id);
    assert_eq!(Some("Test Session".into()), record.title);
    assert_eq!("active", record.status);
    assert_eq!(Some("qwen3-4b".into()), record.model);
    assert_eq!("direct", record.interaction_mode);
    assert_eq!(0, record.current_step);
    assert!(!record.projection_stale);
}

#[test]
fn session_registry_get_by_id() {
    let store = open_test_store();

    store.create_session(NewSessionRecord {
        session_id: "sess_002".into(),
        title: None,
        provider: None,
        model: None,
        base_url: None,
        working_directory: None,
        interaction_mode: "conversational".into(),
    }).unwrap();

    let record = store.get_session("sess_002").unwrap().unwrap();
    assert_eq!("sess_002", record.session_id);
    assert_eq!("conversational", record.interaction_mode);

    // Non-existent session returns None
    assert!(store.get_session("nonexistent").unwrap().is_none());
}

#[test]
fn session_registry_list_orders_by_updated_at_desc() {
    let store = open_test_store();

    // Create sessions with slight delay to ensure ordering
    // Create two sessions. Order should be DESC by updated_at, rowid breaks ties.
    store.create_session(NewSessionRecord {
        session_id: "first".into(),
        title: Some("First".into()),
        provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    store.create_session(NewSessionRecord {
        session_id: "second".into(),
        title: Some("Second".into()),
        provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    let sessions = store.list_sessions(SessionListFilter::default()).unwrap();
    assert_eq!(2, sessions.len());
    // Most recently created/updated should be first
    assert_eq!("second", sessions[0].session_id);
    assert_eq!("first", sessions[1].session_id);
}

#[test]
fn session_registry_archive_hides_from_default_list() {
    let store = open_test_store();

    store.create_session(NewSessionRecord {
        session_id: "visible".into(),
        title: None, provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    store.create_session(NewSessionRecord {
        session_id: "to_archive".into(),
        title: None, provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    // Archive one
    store.archive_session("to_archive").unwrap();

    // Default list excludes archived
    let active = store.list_sessions(SessionListFilter::default()).unwrap();
    assert_eq!(1, active.len());
    assert_eq!("visible", active[0].session_id);
}

#[test]
fn session_registry_include_archived_when_requested() {
    let store = open_test_store();

    store.create_session(NewSessionRecord {
        session_id: "archived_one".into(),
        title: None, provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    store.archive_session("archived_one").unwrap();

    let all = store.list_sessions(SessionListFilter {
        include_archived: true,
        limit: None,
    }).unwrap();

    assert_eq!(1, all.len());
    assert_eq!("archived_one", all[0].session_id);
    assert_eq!("archived", all[0].status);
}

#[test]
fn session_registry_update_preview_and_last_trace() {
    let store = open_test_store();

    store.create_session(NewSessionRecord {
        session_id: "sess_update".into(),
        title: None, provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    store.update_session(SessionRegistryUpdate {
        session_id: "sess_update".into(),
        title: Some("Updated Title".into()),
        last_message_preview: Some("Hello, how can I help?".into()),
        last_trace_id: Some("trace_abc".into()),
        last_global_sequence: Some(42),
        current_phase: Some("Inference".into()),
        current_step: Some(3),
        ..Default::default()
    }).unwrap();

    let record = store.get_session("sess_update").unwrap().unwrap();
    assert_eq!(Some("Updated Title".into()), record.title);
    assert_eq!(Some("Hello, how can I help?".into()), record.last_message_preview);
    assert_eq!(Some("trace_abc".into()), record.last_trace_id);
    assert_eq!(Some(42), record.last_global_sequence);
    assert_eq!(Some("Inference".into()), record.current_phase);
    assert_eq!(3, record.current_step);
}

#[test]
fn session_registry_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_registry.db");

    // Create and write
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let store = rt.block_on(SqliteStore::open(SqliteStoreConfig::file(&db_path))).unwrap();
        store.create_session(NewSessionRecord {
            session_id: "persistent".into(),
            title: Some("Survives Reopen".into()),
            provider: None, model: None, base_url: None,
            working_directory: None, interaction_mode: "direct".into(),
        }).unwrap();
    }

    // Reopen and read
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let store = rt.block_on(SqliteStore::open(SqliteStoreConfig::file(&db_path))).unwrap();
        let record = store.get_session("persistent").unwrap().unwrap();
        assert_eq!("persistent", record.session_id);
        assert_eq!(Some("Survives Reopen".into()), record.title);
    }
}

#[test]
fn session_registry_is_not_required_for_trace_replay() {
    // Trace operations work independently of the registry
    let store = open_test_store();

    // Create a session in the registry
    store.create_session(NewSessionRecord {
        session_id: "trace_test".into(),
        title: None, provider: None, model: None, base_url: None,
        working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    // Archive it
    store.archive_session("trace_test").unwrap();

    // The trace store still works — registry status doesn't affect trace
    let rt = runtime();
    let seq = rt.block_on(store.current_global_sequence()).unwrap();
    assert_eq!(0, seq);
}
