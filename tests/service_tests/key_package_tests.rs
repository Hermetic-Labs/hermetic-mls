use std::sync::Arc;

use hermetic_mls::{
    db::{DatabaseInterface, KeyPackage},
    service::{
        mls::{
            self, 
            mls_delivery_service_server::MlsDeliveryService,
            PublishKeyPackageRequest, GetKeyPackageRequest, ListKeyPackagesRequest
        },
        MLSServiceImpl,
    },
};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

use crate::mock_db::MockDatabase;

/// Test the PublishKeyPackage RPC
#[tokio::test]
async fn test_publish_key_package() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    // Create a service with validation disabled for testing
    let service = MLSServiceImpl::new_skip_validation(db.clone());
    
    // Create a test request
    let client_id = Uuid::new_v4();
    let key_package_data = vec![1, 2, 3, 4, 5];
    
    let request = Request::new(PublishKeyPackageRequest {
        client_id: client_id.to_string(),
        key_package: key_package_data.clone(),
    });
    
    // Call the service
    let response = service.publish_key_package(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse key_package_id from response
    let key_package_id = Uuid::parse_str(&response.key_package_id).unwrap();
    
    // Verify key package was stored in database
    let key_package = db.get_key_package(key_package_id).await.unwrap();
    assert_eq!(key_package.client_id, client_id);
    assert_eq!(key_package.data, key_package_data);
    assert_eq!(key_package.used, false);
}

/// Test the GetKeyPackage RPC
#[tokio::test]
async fn test_get_key_package() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create a test key package
    let key_package_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();
    let key_package = KeyPackage {
        id: key_package_id,
        client_id,
        data: vec![1, 2, 3, 4, 5],
        created_at: Utc::now(),
        used: false,
    };
    
    // Add it to the mock database
    db.store_key_package(key_package.clone()).await.unwrap();
    
    // Create a request to get the key package
    let request = Request::new(GetKeyPackageRequest {
        key_package_id: key_package_id.to_string(),
    });
    
    // Call the service
    let response = service.get_key_package(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify key package details in response
    let response_key_package = response.key_package.unwrap();
    assert_eq!(response_key_package.id, key_package_id.to_string());
    assert_eq!(response_key_package.client_id, client_id.to_string());
    assert_eq!(response_key_package.data, vec![1, 2, 3, 4, 5]);
    assert_eq!(response_key_package.used, false);
}

/// Test the ListKeyPackages RPC
#[tokio::test]
async fn test_list_key_packages() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let client_id = Uuid::new_v4();
    let other_client_id = Uuid::new_v4();
    
    // Add key packages for the target client
    let key_package1 = KeyPackage {
        id: Uuid::new_v4(),
        client_id,
        data: vec![1, 2, 3, 4, 5],
        created_at: Utc::now(),
        used: false,
    };
    let key_package2 = KeyPackage {
        id: Uuid::new_v4(),
        client_id,
        data: vec![6, 7, 8, 9, 10],
        created_at: Utc::now(),
        used: false,
    };
    
    // Add a key package for a different client
    let key_package3 = KeyPackage {
        id: Uuid::new_v4(),
        client_id: other_client_id,
        data: vec![11, 12, 13, 14, 15],
        created_at: Utc::now(),
        used: false,
    };
    
    // Store key packages in the database
    db.store_key_package(key_package1.clone()).await.unwrap();
    db.store_key_package(key_package2.clone()).await.unwrap();
    db.store_key_package(key_package3.clone()).await.unwrap();
    
    // Create a request to list key packages for the target client
    let request = Request::new(ListKeyPackagesRequest {
        client_id: client_id.to_string(),
    });
    
    // Call the service
    let response = service.list_key_packages(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify key packages in response
    assert_eq!(response.key_packages.len(), 2);
    
    // Verify key package IDs match (without assuming order)
    let response_ids: Vec<String> = response.key_packages.iter().map(|kp| kp.id.clone()).collect();
    assert!(response_ids.contains(&key_package1.id.to_string()));
    assert!(response_ids.contains(&key_package2.id.to_string()));
    assert!(!response_ids.contains(&key_package3.id.to_string()));
} 