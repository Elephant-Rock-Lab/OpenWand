//! Wave 02i — SQLite migration and ranked search conformance tests.

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
