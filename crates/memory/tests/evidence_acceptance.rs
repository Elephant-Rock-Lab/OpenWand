//! Commit 1 — Persist evidence_kind on write.

#[cfg(test)]
mod evidence_kind_persistence {
    use openwand_memory::evidence::EvidenceKind;
    use openwand_memory::in_memory::InMemoryMemoryStore;
    use openwand_memory::memory_store::MemoryStore;
    use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};

    fn make_episode(id: &str, trace_id: &str) -> MemoryEpisode {
        MemoryEpisode {
            episode_id: id.to_string(),
            source_trace_id: trace_id.to_string(),
            session_id: "s1".to_string(),
            event_kind: "message".to_string(),
            role: EpisodeRole::User,
            content: "test".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    fn make_candidate(claim: &str, ep_id: &str) -> CandidateMemory {
        CandidateMemory {
            claim: claim.to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec![ep_id.to_string()],
        }
    }

    #[tokio::test]
    async fn accept_candidate_persists_evidence_kind() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let record = store.accept_candidate(make_candidate("test claim", "ep1")).await.unwrap().unwrap();
        assert_eq!(EvidenceKind::AcceptedClaim, record.evidence_kind);
    }

    #[tokio::test]
    async fn accept_candidate_defaults_to_accepted_claim() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let record = store.accept_candidate(make_candidate("another claim", "ep1")).await.unwrap().unwrap();
        assert!(record.evidence_kind.is_accepted_state());
    }

    #[tokio::test]
    async fn inmemory_roundtrips_evidence_kind() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let _record = store.accept_candidate(make_candidate("roundtrip test", "ep1")).await.unwrap().unwrap();

        // Retrieve via list_active_records
        let records = store.list_active_records().await.unwrap();
        assert_eq!(1, records.len());
        assert_eq!(EvidenceKind::AcceptedClaim, records[0].evidence_kind);
    }

    #[tokio::test]
    async fn legacy_records_default_without_breakage() {
        // MemoryRecord default evidence_kind is AcceptedClaim
        // This means deserialized records without the field get AcceptedClaim
        let json = r#"{"record_id":"m1","claim":"test","kind":"Fact","confidence":0.9,"source_episode_ids":[],"source_trace_ids":[],"created_at":"2024-01-01T00:00:00Z","valid_until":null,"superseded_by":null}"#;
        let record: openwand_memory::types::MemoryRecord = serde_json::from_str(json).unwrap();
        assert_eq!(EvidenceKind::AcceptedClaim, record.evidence_kind);
    }
}

#[cfg(feature = "sqlite-testing")]
mod sqlite_evidence_kind {
    use openwand_memory::evidence::EvidenceKind;
    use openwand_memory::memory_store::MemoryStore;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
    use tempfile::TempDir;

    fn make_episode(id: &str, trace_id: &str) -> MemoryEpisode {
        MemoryEpisode {
            episode_id: id.to_string(),
            source_trace_id: trace_id.to_string(),
            session_id: "s1".to_string(),
            event_kind: "message".to_string(),
            role: EpisodeRole::User,
            content: "test".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    fn make_candidate(claim: &str, ep_id: &str) -> CandidateMemory {
        CandidateMemory {
            claim: claim.to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec![ep_id.to_string()],
        }
    }

    fn make_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_roundtrips_evidence_kind() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let record = store.accept_candidate(make_candidate("sqlite test", "ep1")).await.unwrap().unwrap();
        assert_eq!(EvidenceKind::AcceptedClaim, record.evidence_kind);

        // Retrieve via list_active_records to verify round-trip through SQLite
        let records = store.list_active_records().await.unwrap();
        assert_eq!(1, records.len());
        assert_eq!(EvidenceKind::AcceptedClaim, records[0].evidence_kind);
    }
}
