use std::sync::Arc;

use hermetic_mls::{
    db::{Client, DatabaseInterface},
    service::{
        mls::{self, mls_delivery_service_server::MlsDeliveryService, RegisterClientRequest},
        MLSServiceImpl,
    },
};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

use crate::mock_db::MockDatabase;

/// Test the RegisterClient RPC
#[tokio::test]
async fn test_register_client() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create a test request
    let user_id = Uuid::new_v4();
    let request = Request::new(RegisterClientRequest {
        user_id: user_id.to_string(),
        identity: "test-identity".to_string(),
        device_name: "test-device".to_string(),
    });
    
    // Call the service
    let response = service.register_client(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse client_id from response
    let client_id = Uuid::parse_str(&response.client_id).unwrap();
    
    // Verify client was stored in database
    let client = db.get_client(client_id).await.unwrap();
    assert_eq!(client.user_id, user_id);
    assert_eq!(client.scheme, "basic");
    assert_eq!(client.device_name, "test-device");
    // We don't assert on credential as it's now generated from identity
}

/// Test the GetClient RPC
#[tokio::test]
async fn test_get_client() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create a test client
    let client_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let client = Client {
        id: client_id,
        user_id,
        credential: vec![1, 2, 3, 4],
        scheme: "basic".to_string(),
        device_name: "test-device".to_string(),
        last_seen: Utc::now(),
        created_at: Utc::now(),
        init_key: Some(vec![1, 2, 3, 4]),
    };
    
    // Add it to the mock database
    db.register_client(client.clone()).await.unwrap();
    
    // Create a request to get the client
    let request = Request::new(mls::GetClientRequest {
        client_id: client_id.to_string(),
    });
    
    // Call the service
    let response = service.get_client(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify client details in response
    let response_client = response.client.unwrap();
    assert_eq!(response_client.id, client_id.to_string());
    assert_eq!(response_client.user_id, user_id.to_string());
    assert_eq!(response_client.credential, vec![1, 2, 3, 4]);
    assert_eq!(response_client.scheme, "basic");
    assert_eq!(response_client.device_name, "test-device");
}

/// Test the ListClients RPC
#[tokio::test]
async fn test_list_clients() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();
    
    // Add clients for the target user
    let client1 = Client {
        id: Uuid::new_v4(),
        user_id,
        credential: vec![1, 2, 3, 4],
        scheme: "basic".to_string(),
        device_name: "device-1".to_string(),
        last_seen: Utc::now(),
        created_at: Utc::now(),
        init_key: Some(vec![5, 6, 7, 8]),
    };
    let client2 = Client {
        id: Uuid::new_v4(),
        user_id,
        credential: vec![5, 6, 7, 8],
        scheme: "basic".to_string(),
        device_name: "device-2".to_string(),
        last_seen: Utc::now(),
        created_at: Utc::now(),
        init_key: Some(vec![9, 10, 11, 12]),
    };
    
    // Add a client for a different user
    let client3 = Client {
        id: Uuid::new_v4(),
        user_id: other_user_id,
        credential: vec![9, 10, 11, 12],
        scheme: "basic".to_string(),
        device_name: "other-device".to_string(),
        last_seen: Utc::now(),
        created_at: Utc::now(),
        init_key: Some(vec![13, 14, 15, 16]),
    };
    
    // Store clients in the database
    db.register_client(client1.clone()).await.unwrap();
    db.register_client(client2.clone()).await.unwrap();
    db.register_client(client3.clone()).await.unwrap();
    
    // Create a request to list clients for the target user
    let request = Request::new(mls::ListClientsRequest {
        user_id: user_id.to_string(),
    });
    
    // Call the service
    let response = service.list_clients(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify clients in response
    assert_eq!(response.clients.len(), 2);
    
    // Verify client IDs match (without assuming order)
    let response_ids: Vec<String> = response.clients.iter().map(|c| c.id.clone()).collect();
    assert!(response_ids.contains(&client1.id.to_string()));
    assert!(response_ids.contains(&client2.id.to_string()));
    assert!(!response_ids.contains(&client3.id.to_string()));
} 