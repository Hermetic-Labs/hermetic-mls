// Mock database for testing
pub mod mock_db;

// Service tests
pub mod service_tests;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use hermetic_mls::service::MLSServiceImpl;
    use crate::mock_db::MockDatabase;

    #[test]
    fn it_works() {
        // Basic sanity test
        let db = Arc::new(MockDatabase::new());
        let _service = MLSServiceImpl::new(db);
        assert!(true);
    }
} 