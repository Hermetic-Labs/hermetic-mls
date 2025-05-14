use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use hermetic_mls::db::{Client, DatabaseInterface, DbError, DbResult, Group, KeyPackage, Membership, Message};
use uuid::Uuid;

/// A mock database implementation for testing
pub struct MockDatabase {
    clients: Mutex<HashMap<Uuid, Client>>,
    key_packages: Mutex<HashMap<Uuid, KeyPackage>>,
    groups: Mutex<HashMap<Uuid, Group>>,
    memberships: Mutex<HashMap<Uuid, Membership>>,
    messages: Mutex<HashMap<Uuid, Message>>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            key_packages: Mutex::new(HashMap::new()),
            groups: Mutex::new(HashMap::new()),
            memberships: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DatabaseInterface for MockDatabase {
    // Client operations
    async fn register_client(&self, client: Client) -> DbResult<()> {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client.id, client);
        Ok(())
    }

    async fn get_client(&self, client_id: Uuid) -> DbResult<Client> {
        let clients = self.clients.lock().unwrap();
        clients
            .get(&client_id)
            .cloned()
            .ok_or(DbError::NotFound)
    }

    async fn list_clients_by_user(&self, user_id: Uuid) -> DbResult<Vec<Client>> {
        let clients = self.clients.lock().unwrap();
        let filtered_clients: Vec<Client> = clients
            .values()
            .filter(|client| client.user_id == user_id)
            .cloned()
            .collect();
        Ok(filtered_clients)
    }

    async fn update_client_last_seen(&self, client_id: Uuid) -> DbResult<()> {
        let mut clients = self.clients.lock().unwrap();
        if let Some(client) = clients.get_mut(&client_id) {
            client.last_seen = Utc::now();
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }

    // KeyPackage operations
    async fn store_key_package(&self, key_package: KeyPackage) -> DbResult<()> {
        let mut key_packages = self.key_packages.lock().unwrap();
        key_packages.insert(key_package.id, key_package);
        Ok(())
    }

    async fn get_key_package(&self, key_package_id: Uuid) -> DbResult<KeyPackage> {
        let key_packages = self.key_packages.lock().unwrap();
        key_packages
            .get(&key_package_id)
            .cloned()
            .ok_or(DbError::NotFound)
    }

    async fn list_key_packages_by_client(&self, client_id: Uuid) -> DbResult<Vec<KeyPackage>> {
        let key_packages = self.key_packages.lock().unwrap();
        let filtered_packages: Vec<KeyPackage> = key_packages
            .values()
            .filter(|kp| kp.client_id == client_id)
            .cloned()
            .collect();
        Ok(filtered_packages)
    }

    async fn mark_key_package_used(&self, key_package_id: Uuid) -> DbResult<()> {
        let mut key_packages = self.key_packages.lock().unwrap();
        if let Some(key_package) = key_packages.get_mut(&key_package_id) {
            key_package.used = true;
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }

    // Group operations
    async fn create_group(&self, group: Group) -> DbResult<()> {
        let mut groups = self.groups.lock().unwrap();
        groups.insert(group.id, group);
        Ok(())
    }

    async fn get_group(&self, group_id: Uuid) -> DbResult<Group> {
        let groups = self.groups.lock().unwrap();
        groups
            .get(&group_id)
            .cloned()
            .ok_or(DbError::NotFound)
    }

    async fn list_groups_by_client(&self, client_id: Uuid) -> DbResult<Vec<Group>> {
        let groups = self.groups.lock().unwrap();
        let memberships = self.memberships.lock().unwrap();
        
        // Find group IDs where this client is a member
        let client_group_ids: Vec<Uuid> = memberships
            .values()
            .filter(|m| m.client_id == client_id && m.removed_at.is_none())
            .map(|m| m.group_id)
            .collect();
        
        // Get the groups
        let client_groups: Vec<Group> = groups
            .values()
            .filter(|g| client_group_ids.contains(&g.id))
            .cloned()
            .collect();
        
        Ok(client_groups)
    }

    async fn update_group_epoch(&self, group_id: Uuid, epoch: i64) -> DbResult<()> {
        let mut groups = self.groups.lock().unwrap();
        if let Some(group) = groups.get_mut(&group_id) {
            group.epoch = epoch;
            group.updated_at = Utc::now();
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }

    async fn update_group_state(&self, group_id: Uuid, state: Vec<u8>) -> DbResult<()> {
        let mut groups = self.groups.lock().unwrap();
        if let Some(group) = groups.get_mut(&group_id) {
            group.state = Some(state);
            group.updated_at = Utc::now();
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }

    // Membership operations
    async fn add_membership(&self, membership: Membership) -> DbResult<()> {
        let mut memberships = self.memberships.lock().unwrap();
        memberships.insert(membership.id, membership);
        Ok(())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> DbResult<()> {
        let mut memberships = self.memberships.lock().unwrap();
        if let Some(membership) = memberships.get_mut(&membership_id) {
            membership.removed_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }

    async fn list_memberships_by_group(&self, group_id: Uuid) -> DbResult<Vec<Membership>> {
        let memberships = self.memberships.lock().unwrap();
        let filtered_memberships: Vec<Membership> = memberships
            .values()
            .filter(|m| m.group_id == group_id)
            .cloned()
            .collect();
        Ok(filtered_memberships)
    }

    async fn list_memberships_by_client(&self, client_id: Uuid) -> DbResult<Vec<Membership>> {
        let memberships = self.memberships.lock().unwrap();
        let filtered_memberships: Vec<Membership> = memberships
            .values()
            .filter(|m| m.client_id == client_id)
            .cloned()
            .collect();
        Ok(filtered_memberships)
    }

    // Message operations
    async fn store_message(&self, message: Message) -> DbResult<()> {
        let mut messages = self.messages.lock().unwrap();
        messages.insert(message.id, message);
        Ok(())
    }

    async fn fetch_messages_for_client(&self, client_id: Uuid, group_id: Option<Uuid>, include_read: bool) -> DbResult<Vec<Message>> {
        // First get all groups this client is a member of
        let memberships = self.memberships.lock().unwrap();
        let client_group_ids: Vec<Uuid> = memberships
            .values()
            .filter(|m| m.client_id == client_id && m.removed_at.is_none())
            .map(|m| m.group_id)
            .collect();
        
        // Filter messages
        let messages = self.messages.lock().unwrap();
        let mut filtered_messages: Vec<Message> = Vec::new();
        
        for message in messages.values() {
            // Apply group filter if provided
            if let Some(filter_group_id) = group_id {
                if message.group_id != filter_group_id {
                    continue;
                }
            } else if !client_group_ids.contains(&message.group_id) {
                // Skip messages for groups the client is not a member of
                continue;
            }
            
            // Apply read filter
            if !include_read && message.read {
                continue;
            }
            
            filtered_messages.push(message.clone());
        }
        
        Ok(filtered_messages)
    }

    async fn mark_messages_read(&self, message_ids: Vec<Uuid>) -> DbResult<()> {
        let mut messages = self.messages.lock().unwrap();
        for id in message_ids {
            if let Some(message) = messages.get_mut(&id) {
                message.read = true;
            }
        }
        Ok(())
    }
} 