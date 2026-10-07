use super::*;

#[test]
fn remote_auto_indexing_requires_remote_feature() {
    {
        let _remote_flag = FeatureFlag::RemoteCodebaseIndexing.override_enabled(false);
        let _flag = FeatureFlag::FullSourceCodeEmbedding.override_enabled(true);
        assert!(!codebase_auto_indexing_enabled(
            CodebaseAutoIndexingSurface::Remote,
            true,
            true,
        ));
    }
    {
        let _remote_flag = FeatureFlag::RemoteCodebaseIndexing.override_enabled(true);
        let _flag = FeatureFlag::FullSourceCodeEmbedding.override_enabled(true);
        assert!(codebase_auto_indexing_enabled(
            CodebaseAutoIndexingSurface::Remote,
            true,
            true,
        ));
    }
}

#[test]
fn candidate_roots_are_deduped_before_filtering() {
    let roots = vec!["/repo", "/repo", "/other"];
    let candidates = auto_index_candidate_roots(roots, |root| *root != "/other");

    assert_eq!(candidates, vec!["/repo"]);
}
