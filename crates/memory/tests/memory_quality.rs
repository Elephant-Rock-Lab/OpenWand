//! Wave 02i — SQLite migration and ranked search conformance tests.
//! Wave 02i-b commit 2 — Migration 0003 roundtrip tests.

#[cfg(feature = "sqlite")]
mod sqlite_quality {
    use openwand_memory::memory_store::MemoryStore;
    use openwand_memory::query::MemoryQuery;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
    #[cfg(feature = "testing")]
    use openwand_memory::testing::HeuristicExtractor;
    use tempfile::TempDir;

    fn make_episode(id: &str, trace_id: &str, session: &str, role: EpisodeRole, content: &str) -> MemoryEpisode {
        MemoryEpisode {
            episode_id: id.to_string(),
            source_trace_id: trace_id.to_string(),
            session_id: session.to_string(),
            event_kind: "message".to_string(),
            role,
            content: content.to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    fn make_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_migration_adds_ranking_columns() {
        let store = make_store();
        // Verify the new columns exist by inserting a record with them
        let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "test");
        store.project_episode(ep).await.unwrap();

        let candidate = CandidateMemory {
            claim: "Test claim for migration".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".to_string()],
        };
        let result = store.accept_candidate(candidate).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn sqlite_existing_records_survive_migration() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");

        // Create store, insert a record
        {
            let store = SqliteMemoryStore::open(&path).unwrap();
            let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "persistent");
            store.project_episode(ep).await.unwrap();
        }

        // Reopen — migration 0002 should run again (idempotent)
        {
            let store = SqliteMemoryStore::open(&path).unwrap();
            let records = store.list_active_records().await.unwrap();
            // The episode exists but no records were extracted (no extractor run)
            assert_eq!(0, records.len());

            // Verify episodes survived
            let episodes = store.get_episodes("s1").await.unwrap();
            assert_eq!(1, episodes.len());
            assert_eq!("persistent", episodes[0].content);
        }
    }

    #[tokio::test]
    #[cfg(feature = "testing")]
    async fn sqlite_search_returns_results_after_migration() {
        let store = make_store();

        let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I always use Rust for everything");
        store.project_episode(ep).await.unwrap();

        let extractor = HeuristicExtractor;
        store.extract_and_accept(&extractor).await.unwrap();

        let ctx = store
            .search_records(MemoryQuery::new("Rust"))
            .await
            .unwrap();

        assert!(ctx.total_hits > 0, "should find results about Rust");
    }
}

/// Migration 0003 roundtrip tests.
#[cfg(feature = "sqlite")]
mod migration_0003 {
    use openwand_memory::memory_store::MemoryStore;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
    use tempfile::TempDir;

    fn make_episode(id: &str, trace_id: &str, session: &str, role: EpisodeRole, content: &str) -> MemoryEpisode {
        MemoryEpisode {
            episode_id: id.to_string(),
            source_trace_id: trace_id.to_string(),
            session_id: session.to_string(),
            event_kind: "message".to_string(),
            role,
            content: content.to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    fn make_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn sqlite_migration_0003_is_additive() {
        // If the store opens without error, all migrations ran successfully.
        let _store = make_store();
    }

    #[tokio::test]
    async fn sqlite_existing_records_default_to_accepted_claim() {
        let store = make_store();

        // Insert a record through the existing interface
        let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I use Rust");
        store.project_episode(ep).await.unwrap();

        let candidate = CandidateMemory {
            claim: "User prefers Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".to_string()],
        };
        let record = store.accept_candidate(candidate).await.unwrap();
        assert!(record.is_some());

        // Verify evidence_kind is populated by accept_candidate
        let conn = store.conn_for_test();
        let evidence_kind: Option<String> = conn
            .query_row(
                "SELECT evidence_kind FROM memory_record WHERE record_id = ?1",
                rusqlite::params![record.as_ref().unwrap().record_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(Some("AcceptedClaim".to_string()), evidence_kind, "new records should have AcceptedClaim evidence_kind");
    }

    #[test]
    fn sqlite_evidence_kind_roundtrips() {
        let store = make_store();
        let conn = store.conn_for_test();

        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, evidence_kind)
             VALUES ('test1', 'fact', 'test claim', 9000, 'active', 0, 0, 0, 'UserStatedClaim')",
            [],
        ).unwrap();

        let kind: String = conn
            .query_row(
                "SELECT evidence_kind FROM memory_record WHERE record_id = 'test1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("UserStatedClaim", kind);
    }

    #[test]
    fn sqlite_normalized_text_hash_roundtrips() {
        let store = make_store();
        let conn = store.conn_for_test();

        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, normalized_text_hash)
             VALUES ('test2', 'fact', 'test', 9000, 'active', 0, 0, 0, 'abc123hash')",
            [],
        ).unwrap();

        let hash: String = conn
            .query_row(
                "SELECT normalized_text_hash FROM memory_record WHERE record_id = 'test2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("abc123hash", hash);
    }

    #[test]
    fn sqlite_conflict_group_id_roundtrips() {
        let store = make_store();
        let conn = store.conn_for_test();

        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, conflict_group_id)
             VALUES ('test3', 'fact', 'test', 9000, 'active', 0, 0, 0, 'cg_001')",
            [],
        ).unwrap();

        let group: String = conn
            .query_row(
                "SELECT conflict_group_id FROM memory_record WHERE record_id = 'test3'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("cg_001", group);
    }

    #[test]
    fn sqlite_supersedes_record_id_roundtrips() {
        let store = make_store();
        let conn = store.conn_for_test();

        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, supersedes_record_id)
             VALUES ('test4', 'fact', 'test', 9000, 'active', 0, 0, 0, 'mem_old')",
            [],
        ).unwrap();

        let sup: String = conn
            .query_row(
                "SELECT supersedes_record_id FROM memory_record WHERE record_id = 'test4'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("mem_old", sup);
    }
}
