use std::sync::Arc;

use openmls::prelude::{KeyPackageIn, OpenMlsProvider, OpenMlsCrypto, OpenMlsRand};
use openmls::credentials::{BasicCredential, Credential};
use openmls_rust_crypto::OpenMlsRustCrypto;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use tls_codec::{Serialize as TlsSerialize, Deserialize as TlsDeserialize};

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
    crypto: OpenMlsRustCrypto,
    skip_validation: bool,
}

impl<DB: DatabaseInterface> MLSServiceImpl<DB> {
    pub fn new(db: Arc<DB>) -> Self {
        let crypto = OpenMlsRustCrypto::default();
        Self { db, crypto, skip_validation: false }
    }
    
    // Create a test version that skips validation
    // Note: No cfg(test) attribute so it's available for both tests and normal code
    pub fn new_skip_validation(db: Arc<DB>) -> Self {
        let crypto = OpenMlsRustCrypto::default();
        Self { db, crypto, skip_validation: true }
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
    
    // Validate an MLS key package using OpenMLS
    fn validate_key_package(&self, key_package_bytes: &[u8]) -> Result<(), Status> {
        // Skip validation if flag is set (for testing)
        if self.skip_validation {
            return Ok(());
        }
        
        use openmls::versions::ProtocolVersion;
        use openmls::prelude::tls_codec::Deserialize;
        
        if key_package_bytes.is_empty() {
            return Err(Status::invalid_argument("Empty key package"));
        }

        // First deserialize the bytes to a KeyPackageIn
        let key_package_in = match KeyPackageIn::tls_deserialize(&mut &key_package_bytes[..]) {
            Ok(kp) => kp,
            Err(e) => return Err(Status::invalid_argument(format!("Invalid key package format: {}", e))),
        };

        // Then validate the KeyPackageIn to get a validated KeyPackage
        match key_package_in.validate(self.crypto.crypto(), ProtocolVersion::Mls10) {
            Ok(_) => Ok(()),
            Err(e) => Err(Status::invalid_argument(format!("Key package validation failed: {}", e))),
        }
    }
    
    // Validate MLS group state
    fn validate_group_state(&self, group_state_bytes: &[u8]) -> Result<(), Status> {
        // Skip validation if flag is set (for testing)
        if self.skip_validation {
            return Ok(());
        }
        
        if group_state_bytes.is_empty() {
            return Err(Status::invalid_argument("Empty group state"));
        }
        
        // Group state validation would normally require more context
        // such as ciphersuites and provider setup
        // This is a simplified validation that just checks if data is present
        
        Ok(())
    }

    // Validate an MLS proposal
    fn validate_proposal(&self, proposal_bytes: &[u8]) -> Result<(), Status> {
        // Skip validation if flag is set (for testing)
        if self.skip_validation {
            return Ok(());
        }
        
        if proposal_bytes.is_empty() {
            return Err(Status::invalid_argument("Empty proposal"));
        }

        // Basic check for now - full validation would need MlsGroup context
        // which would require building a proper MLS context
        Ok(())
    }
    
    // Validate an MLS commit
    fn validate_commit(&self, commit_bytes: &[u8]) -> Result<(), Status> {
        // Skip validation if flag is set (for testing)
        if self.skip_validation {
            return Ok(());
        }
        
        if commit_bytes.is_empty() {
            return Err(Status::invalid_argument("Empty commit"));
        }

        // Basic check for now - full validation would need MlsGroup context
        // which would require building a proper MLS context
        Ok(())
    }
    
    // Validate an MLS welcome message
    fn validate_welcome(&self, welcome_bytes: &[u8]) -> Result<(), Status> {
        // Skip validation if flag is set (for testing)
        if self.skip_validation {
            return Ok(());
        }
        
        if welcome_bytes.is_empty() {
            return Err(Status::invalid_argument("Empty welcome message"));
        }

        // Basic check for now - full validation would need additional context
        Ok(())
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
        
        // Generate a BasicCredential using the identity
        let identity = req.identity.as_bytes().to_vec();
        let basic_credential = BasicCredential::new(identity);
        
        // Convert to Credential (from trait implementation)
        let credential: Credential = basic_credential.into();
        
        // Serialize the credential for storage
        let credential_bytes = credential.tls_serialize_detached()
            .map_err(|e| Status::internal(
                format!("Failed to serialize credential: {}", e)
            ))?;
        
        // Generate random bytes for key derivation
        let random_bytes = self.crypto.rand().random_vec(32)
            .map_err(|e| Status::internal(
                format!("Failed to generate random bytes: {}", e)
            ))?;
        
        // Generate an initial HPKE key pair for the client using derive_hpke_keypair
        let key_pair = self.crypto.crypto().derive_hpke_keypair(
            openmls::prelude::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519.hpke_config(),
            &random_bytes
        ).map_err(|e| Status::internal(
            format!("Failed to derive HPKE key pair: {}", e)
        ))?;
        
        // Serialize the init_key for storage
        let init_key_bytes = key_pair.public.tls_serialize_detached()
            .map_err(|e| Status::internal(
                format!("Failed to serialize init key: {}", e)
            ))?;
        
        let client = crate::db::Client {
            id: client_id,
            user_id,
            credential: credential_bytes,
            scheme: "basic".to_string(),  // Set to "basic" since we're generating a BasicCredential
            device_name: req.device_name,
            last_seen: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            init_key: Some(init_key_bytes),
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
        
        // Get client data from database
        let client = self.db.get_client(client_id)
            .await
            .map_err(Self::map_db_error)?;
        
        // Deserialize the credential using TlsDeserialize trait
        let mut credential_slice = client.credential.as_slice();
        let credential = Credential::tls_deserialize(&mut credential_slice)
            .map_err(|e| Status::internal(
            format!("Failed to deserialize credential: {}", e)
        ))?;
        
        // Generate random bytes for key derivation
        let random_bytes = self.crypto.rand().random_vec(32)
            .map_err(|e| Status::internal(
                format!("Failed to generate random bytes: {}", e)
            ))?;
        
        // Generate a fresh HPKE key pair for this key package using derive_hpke_keypair
        let hpke_keypair = self.crypto.crypto().derive_hpke_keypair(
            openmls::prelude::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519.hpke_config(),
            &random_bytes
        ).map_err(|e| Status::internal(
            format!("Failed to derive HPKE key pair: {}", e)
        ))?;
        
        // Get the public key to use as init key
        let init_key = hpke_keypair.public.clone();
        
        // Select ciphersuite (could be made configurable in the future)
        let ciphersuite = openmls::prelude::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
        
        // Import necessary types for key package creation
        use openmls::credentials::CredentialWithKey;
        use openmls::key_packages::KeyPackage;
        use openmls_basic_credential::SignatureKeyPair;
        
        // To create a key package we need a signature key
        let signature_key = SignatureKeyPair::new(ciphersuite.signature_algorithm())
            .map_err(|e| Status::internal(
                format!("Failed to generate signature key pair: {}", e)
            ))?;
        
        // Create the credential with key
        let credential_with_key = CredentialWithKey {
            credential,
            signature_key: signature_key.public().into(),
        };
        
        // Create a KeyPackage using the OpenMLS SDK
        let key_package_bundle = KeyPackage::builder()
            .build(
                ciphersuite,
                &self.crypto,
                &signature_key,
                credential_with_key,
            )
            .map_err(|e| Status::internal(
                format!("Failed to build key package: {}", e)
            ))?;
        
        // Serialize the key package for storage
        let key_package_bytes = key_package_bundle.key_package().tls_serialize_detached()
            .map_err(|e| Status::internal(
                format!("Failed to serialize key package: {}", e)
            ))?;
        
        // Create key package record
        let key_package_id = Uuid::new_v4();
        let key_package_record = crate::db::KeyPackage {
            id: key_package_id,
            client_id,
            data: key_package_bytes,
            created_at: chrono::Utc::now(),
            used: false,
            // In a production system, you would store the private key securely
            // This might require extending the KeyPackage struct to include a private_key field
        };
        
        // Store in database
        self.db.store_key_package(key_package_record)
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
        let group_state = req.initial_state.clone();
        self.validate_group_state(&group_state)?;
        
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
        
        // Validate the proposal
        self.validate_proposal(&req.proposal)?;
        
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
        
        // Validate the commit
        self.validate_commit(&req.commit)?;
        
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
        
        // Validate the welcome
        self.validate_welcome(&req.welcome)?;
        
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