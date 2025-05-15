// Mock database for testing
pub mod mock_db;

// Service tests
pub mod service_tests;

#[cfg(test)]
mod tests {
    use crate::mock_db::MockDatabase;
    use hermetic_mls::service::MLSServiceImpl;
    use std::sync::Arc;

    #[test]
    fn it_works() {
        // Basic sanity test
        let db = Arc::new(MockDatabase::new());
        let _service = MLSServiceImpl::new_skip_validation(db);
        assert!(true);
    }
}
