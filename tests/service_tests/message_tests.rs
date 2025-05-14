use std::sync::Arc;

use mls_ds::{
    db::{DatabaseInterface, Group, Membership, Message},
    service::{
        mls::{
            self, 
            mls_delivery_service_server::MlsDeliveryService,
            StoreProposalRequest, StoreCommitRequest, StoreWelcomeRequest, FetchMessagesRequest
        },
        MLSServiceImpl,
    },
};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::Utc;

use crate::mock_db::MockDatabase;

/// Test the StoreProposal RPC
#[tokio::test]
async fn test_store_proposal() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let group_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let proposal_data = vec![1, 2, 3, 4, 5];
    
    // Create a request to store a proposal
    let request = Request::new(StoreProposalRequest {
        group_id: group_id.to_string(),
        sender_id: sender_id.to_string(),
        proposal: proposal_data.clone(),
        proposal_type: "add".to_string(),
    });
    
    // Call the service
    let response = service.store_proposal(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse message_id from response
    let message_id = Uuid::parse_str(&response.message_id).unwrap();
    
    // Verify message was stored in database
    let messages = db.fetch_messages_for_client(sender_id, Some(group_id), true).await.unwrap();
    
    // Find our message
    let message = messages.iter().find(|m| m.id == message_id).expect("Message not found");
    assert_eq!(message.group_id, group_id);
    assert_eq!(message.sender_id, sender_id);
    assert_eq!(message.message_type, "proposal");
    assert_eq!(message.proposal, Some(proposal_data));
    assert_eq!(message.commit, None);
    assert_eq!(message.welcome, None);
}

/// Test the StoreCommit RPC
#[tokio::test]
async fn test_store_commit() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let group_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let commit_data = vec![1, 2, 3, 4, 5];
    
    // Create a group first with epoch 0
    let group = Group {
        id: group_id,
        creator_id: sender_id,
        epoch: 0,
        state: Some(vec![10, 11, 12]),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };
    
    // Store the group
    db.create_group(group).await.unwrap();
    
    // Create a request to store a commit
    let request = Request::new(StoreCommitRequest {
        group_id: group_id.to_string(),
        sender_id: sender_id.to_string(),
        commit: commit_data.clone(),
        epoch: 1, // New epoch
    });
    
    // Call the service
    let response = service.store_commit(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse message_id from response
    let message_id = Uuid::parse_str(&response.message_id).unwrap();
    
    // Verify message was stored in database
    let messages = db.fetch_messages_for_client(sender_id, Some(group_id), true).await.unwrap();
    
    // Find our message
    let message = messages.iter().find(|m| m.id == message_id).expect("Message not found");
    assert_eq!(message.group_id, group_id);
    assert_eq!(message.sender_id, sender_id);
    assert_eq!(message.message_type, "commit");
    assert_eq!(message.commit, Some(commit_data));
    assert_eq!(message.proposal, None);
    assert_eq!(message.welcome, None);
    
    // Verify the group's epoch was updated
    let updated_group = db.get_group(group_id).await.unwrap();
    assert_eq!(updated_group.epoch, 1);
}

/// Test the StoreWelcome RPC
#[tokio::test]
async fn test_store_welcome() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let group_id = Uuid::new_v4();
    let sender_id = Uuid::new_v4();
    let recipient1_id = Uuid::new_v4();
    let recipient2_id = Uuid::new_v4();
    let welcome_data = vec![1, 2, 3, 4, 5];
    
    // Create a request to store a welcome
    let request = Request::new(StoreWelcomeRequest {
        group_id: group_id.to_string(),
        sender_id: sender_id.to_string(),
        welcome: welcome_data.clone(),
        recipient_ids: vec![
            recipient1_id.to_string(),
            recipient2_id.to_string(),
        ],
    });
    
    // Call the service
    let response = service.store_welcome(request).await.unwrap();
    let response = response.into_inner();
    
    // Parse message_id from response
    let message_id = Uuid::parse_str(&response.message_id).unwrap();
    
    // Verify message was stored in database
    let messages = db.fetch_messages_for_client(sender_id, Some(group_id), true).await.unwrap();
    
    // Find our message
    let message = messages.iter().find(|m| m.id == message_id).expect("Message not found");
    assert_eq!(message.group_id, group_id);
    assert_eq!(message.sender_id, sender_id);
    assert_eq!(message.message_type, "welcome");
    assert_eq!(message.welcome, Some(welcome_data));
    assert_eq!(message.proposal, None);
    assert_eq!(message.commit, None);
}

/// Test the FetchMessages RPC
#[tokio::test]
async fn test_fetch_messages() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());
    
    // Create test data
    let group_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();
    
    // Add client to group via membership
    let membership = Membership {
        id: Uuid::new_v4(),
        client_id,
        group_id,
        role: "member".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };
    db.add_membership(membership).await.unwrap();
    
    // Create some messages
    let message1 = Message {
        id: Uuid::new_v4(),
        group_id,
        sender_id: Uuid::new_v4(),
        created_at: Utc::now(),
        read: false,
        message_type: "proposal".to_string(),
        proposal: Some(vec![1, 2, 3]),
        commit: None,
        welcome: None,
        proposal_type: Some("add".to_string()),
        epoch: None,
        recipients: None,
    };
    
    let message2 = Message {
        id: Uuid::new_v4(),
        group_id,
        sender_id: Uuid::new_v4(),
        created_at: Utc::now(),
        read: true, // This one is already read
        message_type: "commit".to_string(),
        proposal: None,
        commit: Some(vec![4, 5, 6]),
        welcome: None,
        proposal_type: None,
        epoch: Some(1),
        recipients: None,
    };
    
    // Store messages
    db.store_message(message1.clone()).await.unwrap();
    db.store_message(message2.clone()).await.unwrap();
    
    // Create a request to fetch messages
    let request = Request::new(FetchMessagesRequest {
        client_id: client_id.to_string(),
        group_id: group_id.to_string(),
        include_read: false, // Only unread messages
    });
    
    // Call the service
    let response = service.fetch_messages(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify messages in response
    assert_eq!(response.messages.len(), 1); // Only the unread message
    
    let fetched_message = &response.messages[0];
    assert_eq!(fetched_message.id, message1.id.to_string());
    assert_eq!(fetched_message.message_type, "proposal");
    
    // Now fetch all messages including read ones
    let request = Request::new(FetchMessagesRequest {
        client_id: client_id.to_string(),
        group_id: group_id.to_string(),
        include_read: true, // Include read messages
    });
    
    let response = service.fetch_messages(request).await.unwrap();
    let response = response.into_inner();
    
    // Verify messages in response
    assert_eq!(response.messages.len(), 2); // Both messages
} 