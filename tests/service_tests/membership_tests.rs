use std::sync::Arc;

use chrono::Utc;
use hermetic_mls::{
    db::{DatabaseInterface, Group, Membership},
    service::{
        mls::{
            self, mls_delivery_service_server::MlsDeliveryService, AddMemberRequest,
            ListMembershipsRequest, RemoveMemberRequest,
        },
        MLSServiceImpl,
    },
};
use tonic::Request;
use uuid::Uuid;

use crate::mock_db::MockDatabase;

/// Test the AddMember RPC
#[tokio::test]
async fn test_add_member() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());

    // Create test data
    let group_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    // Create a group first
    let group = Group {
        id: group_id,
        creator_id: Uuid::new_v4(),
        epoch: 0,
        state: Some(vec![1, 2, 3]),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        is_active: true,
    };

    // Store the group in the database
    db.create_group(group).await.unwrap();

    // Create a request to add a member
    let request = Request::new(AddMemberRequest {
        group_id: group_id.to_string(),
        client_id: client_id.to_string(),
        role: "member".to_string(),
    });

    // Call the service
    let response = service.add_member(request).await.unwrap();
    let response = response.into_inner();

    // Parse membership_id from response
    let membership_id = Uuid::parse_str(&response.membership_id).unwrap();

    // Verify membership was stored in database
    let memberships = db.list_memberships_by_group(group_id).await.unwrap();
    assert_eq!(memberships.len(), 1);

    let membership = &memberships[0];
    assert_eq!(membership.id, membership_id);
    assert_eq!(membership.client_id, client_id);
    assert_eq!(membership.group_id, group_id);
    assert_eq!(membership.role, "member");
    assert!(membership.removed_at.is_none());
}

/// Test the RemoveMember RPC
#[tokio::test]
async fn test_remove_member() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());

    // Create test data
    let membership_id = Uuid::new_v4();
    let group_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    let membership = Membership {
        id: membership_id,
        client_id,
        group_id,
        role: "member".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };

    // Store membership in the database
    db.add_membership(membership).await.unwrap();

    // Create a request to remove the member
    let request = Request::new(RemoveMemberRequest {
        membership_id: membership_id.to_string(),
    });

    // Call the service
    let response = service.remove_member(request).await.unwrap();
    let response = response.into_inner();

    assert_eq!(response.success, true);

    // Verify membership was removed in database
    let memberships = db.list_memberships_by_group(group_id).await.unwrap();
    assert_eq!(memberships.len(), 1);

    let membership = &memberships[0];
    assert_eq!(membership.id, membership_id);
    assert!(membership.removed_at.is_some()); // Should have a removal timestamp
}

/// Test the ListMemberships RPC
#[tokio::test]
async fn test_list_memberships() {
    // Create a mock database
    let db = Arc::new(MockDatabase::new());
    let service = MLSServiceImpl::new(db.clone());

    // Create test data
    let group_id = Uuid::new_v4();
    let client1_id = Uuid::new_v4();
    let client2_id = Uuid::new_v4();

    // Create memberships
    let membership1 = Membership {
        id: Uuid::new_v4(),
        client_id: client1_id,
        group_id,
        role: "admin".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };

    let membership2 = Membership {
        id: Uuid::new_v4(),
        client_id: client2_id,
        group_id,
        role: "member".to_string(),
        added_at: Utc::now(),
        removed_at: None,
    };

    // Create a removed membership
    let membership3 = Membership {
        id: Uuid::new_v4(),
        client_id: Uuid::new_v4(),
        group_id,
        role: "member".to_string(),
        added_at: Utc::now(),
        removed_at: Some(Utc::now()),
    };

    // Store memberships in the database
    db.add_membership(membership1.clone()).await.unwrap();
    db.add_membership(membership2.clone()).await.unwrap();
    db.add_membership(membership3.clone()).await.unwrap();

    // Create a request to list memberships
    let request = Request::new(ListMembershipsRequest {
        group_id: group_id.to_string(),
    });

    // Call the service
    let response = service.list_memberships(request).await.unwrap();
    let response = response.into_inner();

    // Verify memberships in response
    assert_eq!(response.memberships.len(), 3); // We include removed memberships in response

    // Verify membership details are correct
    let membership_ids: Vec<String> = response.memberships.iter().map(|m| m.id.clone()).collect();
    assert!(membership_ids.contains(&membership1.id.to_string()));
    assert!(membership_ids.contains(&membership2.id.to_string()));
    assert!(membership_ids.contains(&membership3.id.to_string()));

    // Verify we can find both the admin and member roles
    let roles: Vec<String> = response
        .memberships
        .iter()
        .map(|m| m.role.clone())
        .collect();
    assert!(roles.contains(&"admin".to_string()));
    assert!(roles.contains(&"member".to_string()));
}
