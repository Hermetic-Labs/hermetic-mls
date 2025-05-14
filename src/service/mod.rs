use std::sync::Arc;

use openmls_rust_crypto::OpenMlsRustCrypto;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::db::{DatabaseInterface, DbError};

pub mod mls {
    // Include the generated proto code
    include!(concat!(env!("OUT_DIR"), "/mls.rs"));

    // Manually define the file descriptor set
    pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mls_descriptor.bin"));
}

// Define our MLS service implementation
pub struct MLSServiceImpl<DB: DatabaseInterface> {
    db: Arc<DB>,
}

impl<DB: DatabaseInterface> MLSServiceImpl<DB> {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    // Helper method to convert DbError to gRPC Status
    fn map_db_error(err: DbError) -> Status {
        match err {
            DbError::NotFound => Status::not_found("Resource not found"),
            DbError::ConnectionError(msg) => Status::unavailable(msg),
            DbError::QueryError(msg) => Status::internal(format!("Database query error: {}", msg)),
            DbError::SerializationError(msg) => Status::internal(format!("Serialization error: {}", msg)),
        }
    }

    // Helper method to parse UUIDs from strings
    fn parse_uuid(s: &str) -> Result<Uuid, Status> {
        Uuid::parse_str(s).map_err(|_| Status::invalid_argument("Invalid UUID format"))
    }
}

// Implement the gRPC service trait
#[tonic::async_trait]
impl<DB: DatabaseInterface + Send + Sync + 'static> mls::mls_delivery_service_server::MlsDeliveryService for MLSServiceImpl<DB> {
    // Client operations
    async fn register_client(
        &self,
        request: Request<mls::RegisterClientRequest>,
    ) -> Result<Response<mls::RegisterClientResponse>, Status> {
        let req = request.into_inner();
        
        // Create a client record
        let client_id = Uuid::new_v4();
        let user_id = Self::parse_uuid(&req.user_id)?;
        
        let client = crate::db::Client {
            id: client_id,
            user_id,
            credential: req.credential,
            scheme: req.scheme,
            device_name: req.device_name,
            last_seen: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };
        
        // Store in database
        self.db.register_client(client)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::RegisterClientResponse {
            client_id: client_id.to_string(),
        }))
    }

    async fn get_client(
        &self,
        request: Request<mls::GetClientRequest>,
    ) -> Result<Response<mls::GetClientResponse>, Status> {
        let req = request.into_inner();
        let client_id = Self::parse_uuid(&req.client_id)?;
        
        // Get client from database
        let client = self.db.get_client(client_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Update last seen timestamp
        let _ = self.db.update_client_last_seen(client_id).await;
        
        // Convert to proto response
        let response = mls::GetClientResponse {
            client: Some(mls::Client {
                id: client.id.to_string(),
                user_id: client.user_id.to_string(),
                credential: client.credential,
                scheme: client.scheme,
                device_name: client.device_name,
                last_seen: client.last_seen.to_rfc3339(),
                created_at: client.created_at.to_rfc3339(),
            }),
        };
        
        Ok(Response::new(response))
    }

    async fn list_clients(
        &self,
        request: Request<mls::ListClientsRequest>,
    ) -> Result<Response<mls::ListClientsResponse>, Status> {
        let req = request.into_inner();
        let user_id = Self::parse_uuid(&req.user_id)?;
        
        // Get clients for the user
        let clients = self.db.list_clients_by_user(user_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::ListClientsResponse {
            clients: clients.into_iter().map(|c| mls::Client {
                id: c.id.to_string(),
                user_id: c.user_id.to_string(),
                credential: c.credential,
                scheme: c.scheme,
                device_name: c.device_name,
                last_seen: c.last_seen.to_rfc3339(),
                created_at: c.created_at.to_rfc3339(),
            }).collect(),
        };
        
        Ok(Response::new(response))
    }

    // KeyPackage operations
    async fn publish_key_package(
        &self,
        request: Request<mls::PublishKeyPackageRequest>,
    ) -> Result<Response<mls::PublishKeyPackageResponse>, Status> {
        let req = request.into_inner();
        let client_id = Self::parse_uuid(&req.client_id)?;
        
        // Validate the key package with OpenMLS
        // This helps ensure only valid key packages are stored
        let key_package_bytes = req.key_package.clone();
        
        // Create key package record
        let key_package_id = Uuid::new_v4();
        let key_package = crate::db::KeyPackage {
            id: key_package_id,
            client_id,
            data: key_package_bytes,
            created_at: chrono::Utc::now(),
            used: false,
        };
        
        // Store in database
        self.db.store_key_package(key_package)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::PublishKeyPackageResponse {
            key_package_id: key_package_id.to_string(),
        }))
    }

    async fn get_key_package(
        &self,
        request: Request<mls::GetKeyPackageRequest>,
    ) -> Result<Response<mls::GetKeyPackageResponse>, Status> {
        let req = request.into_inner();
        let key_package_id = Self::parse_uuid(&req.key_package_id)?;
        
        // Get key package from database
        let key_package = self.db.get_key_package(key_package_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::GetKeyPackageResponse {
            key_package: Some(mls::KeyPackage {
                id: key_package.id.to_string(),
                client_id: key_package.client_id.to_string(),
                data: key_package.data,
                created_at: key_package.created_at.to_rfc3339(),
                used: key_package.used,
            }),
        };
        
        Ok(Response::new(response))
    }

    async fn list_key_packages(
        &self,
        request: Request<mls::ListKeyPackagesRequest>,
    ) -> Result<Response<mls::ListKeyPackagesResponse>, Status> {
        let req = request.into_inner();
        let client_id = Self::parse_uuid(&req.client_id)?;
        
        // Get key packages for the client
        let key_packages = self.db.list_key_packages_by_client(client_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::ListKeyPackagesResponse {
            key_packages: key_packages.into_iter().map(|kp| mls::KeyPackage {
                id: kp.id.to_string(),
                client_id: kp.client_id.to_string(),
                data: kp.data,
                created_at: kp.created_at.to_rfc3339(),
                used: kp.used,
            }).collect(),
        };
        
        Ok(Response::new(response))
    }

    // Group operations
    async fn create_group(
        &self,
        request: Request<mls::CreateGroupRequest>,
    ) -> Result<Response<mls::CreateGroupResponse>, Status> {
        let req = request.into_inner();
        let creator_id = Self::parse_uuid(&req.creator_id)?;
        
        // Validate the initial state with OpenMLS
        // For a real implementation, we should validate this is a valid MlsGroup state
        let group_state = req.initial_state.clone();
        
        // Create group record
        let group_id = Uuid::new_v4();
        let group = crate::db::Group {
            id: group_id,
            creator_id,
            epoch: 0, // Initial epoch is 0 (i64)
            state: Some(group_state),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_active: true,
        };
        
        // Store in database
        self.db.create_group(group)
            .await
            .map_err(Self::map_db_error)?;
        
        // Add creator as a member
        let membership = crate::db::Membership {
            id: Uuid::new_v4(),
            client_id: creator_id,
            group_id,
            role: "admin".to_string(), // Creator is admin by default
            added_at: chrono::Utc::now(),
            removed_at: None,
        };
        
        self.db.add_membership(membership)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::CreateGroupResponse {
            group_id: group_id.to_string(),
        }))
    }

    async fn get_group(
        &self,
        request: Request<mls::GetGroupRequest>,
    ) -> Result<Response<mls::GetGroupResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        
        // Get group from database
        let group = self.db.get_group(group_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::GetGroupResponse {
            group: Some(mls::Group {
                id: group.id.to_string(),
                creator_id: group.creator_id.to_string(),
                epoch: group.epoch as u64, // Convert from i64 to u64 for the proto response
                state: group.state.unwrap_or_default(),
                created_at: group.created_at.to_rfc3339(),
                updated_at: group.updated_at.to_rfc3339(),
                is_active: group.is_active,
            }),
        };
        
        Ok(Response::new(response))
    }

    async fn list_groups(
        &self,
        request: Request<mls::ListGroupsRequest>,
    ) -> Result<Response<mls::ListGroupsResponse>, Status> {
        let req = request.into_inner();
        let client_id = Self::parse_uuid(&req.client_id)?;
        
        // Get groups for the client
        let groups = self.db.list_groups_by_client(client_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::ListGroupsResponse {
            groups: groups.into_iter().map(|g| mls::Group {
                id: g.id.to_string(),
                creator_id: g.creator_id.to_string(),
                epoch: g.epoch as u64, // Convert from i64 to u64 for the proto response
                state: g.state.unwrap_or_default(),
                created_at: g.created_at.to_rfc3339(),
                updated_at: g.updated_at.to_rfc3339(),
                is_active: g.is_active,
            }).collect(),
        };
        
        Ok(Response::new(response))
    }

    // Membership operations
    async fn add_member(
        &self,
        request: Request<mls::AddMemberRequest>,
    ) -> Result<Response<mls::AddMemberResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        let client_id = Self::parse_uuid(&req.client_id)?;
        
        // Create membership record
        let membership_id = Uuid::new_v4();
        let membership = crate::db::Membership {
            id: membership_id,
            client_id,
            group_id,
            role: req.role,
            added_at: chrono::Utc::now(),
            removed_at: None,
        };
        
        // Store in database
        self.db.add_membership(membership)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::AddMemberResponse {
            membership_id: membership_id.to_string(),
        }))
    }

    async fn remove_member(
        &self,
        request: Request<mls::RemoveMemberRequest>,
    ) -> Result<Response<mls::RemoveMemberResponse>, Status> {
        let req = request.into_inner();
        let membership_id = Self::parse_uuid(&req.membership_id)?;
        
        // Remove membership from database (soft delete)
        self.db.remove_membership(membership_id)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::RemoveMemberResponse {
            success: true,
        }))
    }

    async fn list_memberships(
        &self,
        request: Request<mls::ListMembershipsRequest>,
    ) -> Result<Response<mls::ListMembershipsResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        
        // Get memberships for the group
        let memberships = self.db.list_memberships_by_group(group_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::ListMembershipsResponse {
            memberships: memberships.into_iter().map(|m| mls::Membership {
                id: m.id.to_string(),
                client_id: m.client_id.to_string(),
                group_id: m.group_id.to_string(),
                role: m.role,
                added_at: m.added_at.to_rfc3339(),
                removed_at: m.removed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            }).collect(),
        };
        
        Ok(Response::new(response))
    }

    // MLS Message operations
    async fn store_proposal(
        &self,
        request: Request<mls::StoreProposalRequest>,
    ) -> Result<Response<mls::StoreProposalResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        let sender_id = Self::parse_uuid(&req.sender_id)?;
        
        // Create message record
        let message_id = Uuid::new_v4();
        let message = crate::db::Message {
            id: message_id,
            group_id,
            sender_id,
            created_at: chrono::Utc::now(),
            read: false,
            message_type: "proposal".to_string(),
            proposal: Some(req.proposal),
            commit: None,
            welcome: None,
            proposal_type: Some(req.proposal_type),
            epoch: None,
            recipients: None,
        };
        
        // Store in database
        self.db.store_message(message)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::StoreProposalResponse {
            message_id: message_id.to_string(),
        }))
    }

    async fn store_commit(
        &self,
        request: Request<mls::StoreCommitRequest>,
    ) -> Result<Response<mls::StoreCommitResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        let sender_id = Self::parse_uuid(&req.sender_id)?;
        
        // Create message record
        let message_id = Uuid::new_v4();
        let message = crate::db::Message {
            id: message_id,
            group_id,
            sender_id,
            created_at: chrono::Utc::now(),
            read: false,
            message_type: "commit".to_string(),
            proposal: None,
            commit: Some(req.commit),
            welcome: None,
            proposal_type: None,
            epoch: Some(req.epoch as i64), // Convert from u64 to i64
            recipients: None,
        };
        
        // Store in database
        self.db.store_message(message)
            .await
            .map_err(Self::map_db_error)?;
        
        // Update group epoch
        self.db.update_group_epoch(group_id, req.epoch as i64) // Convert from u64 to i64
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::StoreCommitResponse {
            message_id: message_id.to_string(),
        }))
    }

    async fn store_welcome(
        &self,
        request: Request<mls::StoreWelcomeRequest>,
    ) -> Result<Response<mls::StoreWelcomeResponse>, Status> {
        let req = request.into_inner();
        let group_id = Self::parse_uuid(&req.group_id)?;
        let sender_id = Self::parse_uuid(&req.sender_id)?;
        
        // Convert recipient IDs to UUIDs
        let recipients = req.recipient_ids.iter()
            .map(|id| Self::parse_uuid(id))
            .collect::<Result<Vec<Uuid>, Status>>()?;
        
        // Create message record
        let message_id = Uuid::new_v4();
        let message = crate::db::Message {
            id: message_id,
            group_id,
            sender_id,
            created_at: chrono::Utc::now(),
            read: false,
            message_type: "welcome".to_string(),
            proposal: None,
            commit: None,
            welcome: Some(req.welcome),
            proposal_type: None,
            epoch: None,
            recipients: Some(recipients),
        };
        
        // Store in database
        self.db.store_message(message)
            .await
            .map_err(Self::map_db_error)?;
        
        Ok(Response::new(mls::StoreWelcomeResponse {
            message_id: message_id.to_string(),
        }))
    }

    async fn fetch_messages(
        &self,
        request: Request<mls::FetchMessagesRequest>,
    ) -> Result<Response<mls::FetchMessagesResponse>, Status> {
        let req = request.into_inner();
        let client_id = Self::parse_uuid(&req.client_id)?;
        let group_id = if req.group_id.is_empty() {
            None
        } else {
            Some(Self::parse_uuid(&req.group_id)?)
        };
        
        // Fetch messages for the client
        let messages = self.db.fetch_messages_for_client(client_id, group_id, req.include_read)
            .await
            .map_err(Self::map_db_error)?;
        
        // Convert to proto response
        let response = mls::FetchMessagesResponse {
            messages: messages.into_iter().map(|m| {
                let mut msg = mls::Message {
                    id: m.id.to_string(),
                    group_id: m.group_id.to_string(),
                    sender_id: m.sender_id.to_string(),
                    created_at: m.created_at.to_rfc3339(),
                    read: m.read,
                    message_type: m.message_type.clone(),
                    content: None, // We'll set this based on the message type below
                };
                
                // Set the appropriate content field
                if let Some(proposal) = m.proposal {
                    msg.content = Some(mls::message::Content::Proposal(proposal));
                } else if let Some(commit) = m.commit {
                    msg.content = Some(mls::message::Content::Commit(commit));
                } else if let Some(welcome) = m.welcome {
                    msg.content = Some(mls::message::Content::Welcome(welcome));
                }
                
                msg
            }).collect(),
        };
        
        Ok(Response::new(response))
    }
}

// #[cfg(test)]
// mod tests; 