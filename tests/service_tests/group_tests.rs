use std::sync::Arc;

use hermetic_mls::{
    db::{Client, DatabaseInterface, Group, Membership},
    service::{
        mls::{
            self, 
            mls_delivery_service_server::MlsDeliveryService,
            CreateGroupRequest, GetGroupRequest, ListGroupsRequest
        },
        MLSServiceImpl,
    },
};
use tonic::Request;
use uuid::Uuid;
use chrono::Utc;

use crate::mock_db::MockDatabase;

/// Test the CreateGroup RPC
#[tokio::test]
async fn test_create_group() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create a test request
    let creator_id = Uuid::new_v4();
    let initial_state = vec![1, 2, 3, 4, 5];
    
    let request = Request::new(CreateGroupRequest {
        creator_id: creator_id.to_string(),
        initial_state: initial_state.clone(),
    });
    
    // Call the service
    let response = service.create_group(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse group_id from response
    let group_id = Uuid::parse_str(&response.group_id).unwrap();
    
    // Verify group was stored in database
    let group = db.get_group(group_id).await.unwrap();
    assert_eq!(group.creator_id, creator_id);
    assert_eq!(group.state, Some(initial_state));
    assert_eq!(group.epoch, 0);
    assert_eq!(group.is_active, true);
}

/// Test the GetGroup RPC
#[tokio::test]
async fn test_get_group() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create a test group
    let group_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let group_state = vec![1, 2, 3, 4, 5];
    
    let group = Group {
        id: group_id,
        creator_id,
        epoch: 0,
        state: Some(group_state.clone()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };
    
    // Add it to the mock database
    db.create_group(group.clone()).await.unwrap();
    
    // Create a request to get the group
    let request = Request::new(GetGroupRequest {
        group_id: group_id.to_string(),
    });
    
    // Call the service
    let response = service.get_group(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify group details in response
    let response_group = response.group.unwrap();
    assert_eq!(response_group.id, group_id.to_string());
    assert_eq!(response_group.creator_id, creator_id.to_string());
    assert_eq!(response_group.epoch, 0);
    assert_eq!(response_group.state, group_state);
    assert_eq!(response_group.is_active, true);
}

/// Test the ListGroups RPC
#[tokio::test]
async fn test_list_groups() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let client_id = Uuid::new_v4();
    let other_client_id = Uuid::new_v4();
    
    // Create two groups
    let group1_id = Uuid::new_v4();
    let group2_id = Uuid::new_v4();
    
    let group1 = Group {
        id: group1_id,
        creator_id: client_id,
        epoch: 0,
        state: Some(vec![1, 2, 3]),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };
    
    let group2 = Group {
        id: group2_id,
        creator_id: other_client_id,
        epoch: 0,
        state: Some(vec![4, 5, 6]),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };
    
    // Store groups in the database
    db.create_group(group1.clone()).await.unwrap();
    db.create_group(group2.clone()).await.unwrap();
    
    // Create memberships to associate client with groups
    let membership1 = Membership {
        id: Uuid::new_v4(),
        client_id,
        group_id: group1_id,
        role: "admin".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };
    
    let membership2 = Membership {
        id: Uuid::new_v4(),
        client_id,
        group_id: group2_id,
        role: "member".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };
    
    // Store memberships
    db.add_membership(membership1).await.unwrap();
    db.add_membership(membership2).await.unwrap();
    
    // Create a request to list groups for the client
    let request = Request::new(ListGroupsRequest {
        client_id: client_id.to_string(),
    });
    
    // Call the service
    let response = service.list_groups(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify groups in response
    assert_eq!(response.groups.len(), 2);
    
    // Verify group IDs match (without assuming order)
    let response_ids: Vec<String> = response.groups.iter().map(|g| g.id.clone()).collect();
    assert!(response_ids.contains(&group1_id.to_string()));
    assert!(response_ids.contains(&group2_id.to_string()));
} 