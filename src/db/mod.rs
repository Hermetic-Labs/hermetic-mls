
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use thiserror::Error;
use uuid::Uuid;

// Define error types
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Resource not found")]
    NotFound,
    
    #[error("Database connection error: {0}")]
    ConnectionError(String),
    
    #[error("Database query error: {0}")]
    QueryError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// Define a common result type for database operations
pub type DbResult<T> = Result<T, DbError>;

// Client data structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Client {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential: Vec<u8>,
    pub scheme: String,
    pub device_name: String,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// KeyPackage data structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KeyPackage {
    pub id: Uuid,
    pub client_id: Uuid,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub used: bool,
}

// Group data structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub epoch: i64, // Changed to i64 for PostgreSQL compatibility
    pub state: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

// Membership data structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Membership {
    pub id: Uuid,
    pub client_id: Uuid,
    pub group_id: Uuid,
    pub role: String,
    pub added_at: DateTime<Utc>,
    pub removed_at: Option<DateTime<Utc>>,
}

// Message data structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub group_id: Uuid,
    pub sender_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub read: bool,
    pub message_type: String,
    pub proposal: Option<Vec<u8>>,
    pub commit: Option<Vec<u8>>,
    pub welcome: Option<Vec<u8>>,
    pub proposal_type: Option<String>,
    pub epoch: Option<i64>, // Changed to i64 for PostgreSQL compatibility
    pub recipients: Option<Vec<Uuid>>,
}

// Define the database interface trait
#[async_trait]
pub trait DatabaseInterface: Send + Sync {
    // Client operations
    async fn register_client(&self, client: Client) -> DbResult<()>;
    async fn get_client(&self, client_id: Uuid) -> DbResult<Client>;
    async fn list_clients_by_user(&self, user_id: Uuid) -> DbResult<Vec<Client>>;
    async fn update_client_last_seen(&self, client_id: Uuid) -> DbResult<()>;
    
    // KeyPackage operations
    async fn store_key_package(&self, key_package: KeyPackage) -> DbResult<()>;
    async fn get_key_package(&self, key_package_id: Uuid) -> DbResult<KeyPackage>;
    async fn list_key_packages_by_client(&self, client_id: Uuid) -> DbResult<Vec<KeyPackage>>;
    async fn mark_key_package_used(&self, key_package_id: Uuid) -> DbResult<()>;
    
    // Group operations
    async fn create_group(&self, group: Group) -> DbResult<()>;
    async fn get_group(&self, group_id: Uuid) -> DbResult<Group>;
    async fn list_groups_by_client(&self, client_id: Uuid) -> DbResult<Vec<Group>>;
    async fn update_group_epoch(&self, group_id: Uuid, epoch: i64) -> DbResult<()>;
    async fn update_group_state(&self, group_id: Uuid, state: Vec<u8>) -> DbResult<()>;
    
    // Membership operations
    async fn add_membership(&self, membership: Membership) -> DbResult<()>;
    async fn remove_membership(&self, membership_id: Uuid) -> DbResult<()>;
    async fn list_memberships_by_group(&self, group_id: Uuid) -> DbResult<Vec<Membership>>;
    async fn list_memberships_by_client(&self, client_id: Uuid) -> DbResult<Vec<Membership>>;
    
    // Message operations
    async fn store_message(&self, message: Message) -> DbResult<()>;
    async fn fetch_messages_for_client(&self, client_id: Uuid, group_id: Option<Uuid>, include_read: bool) -> DbResult<Vec<Message>>;
    async fn mark_messages_read(&self, message_ids: Vec<Uuid>) -> DbResult<()>;
}

// Implementation of the DatabaseInterface trait using SQLx and PostgreSQL
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabaseInterface for PostgresDatabase {
    // Client operations
    async fn register_client(&self, client: Client) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO clients (id, user_id, credential, scheme, device_name, last_seen, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(client.id)
        .bind(client.user_id)
        .bind(client.credential)
        .bind(&client.scheme)
        .bind(&client.device_name)
        .bind(client.last_seen)
        .bind(client.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_client(&self, client_id: Uuid) -> DbResult<Client> {
        let client = sqlx::query_as::<_, Client>(
            r#"
            SELECT * FROM clients
            WHERE id = $1
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?
        .ok_or(DbError::NotFound)?;
        
        Ok(client)
    }
    
    async fn list_clients_by_user(&self, user_id: Uuid) -> DbResult<Vec<Client>> {
        let clients = sqlx::query_as::<_, Client>(
            r#"
            SELECT * FROM clients
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(clients)
    }
    
    async fn update_client_last_seen(&self, client_id: Uuid) -> DbResult<()> {
        let now = Utc::now();
        
        sqlx::query(
            r#"
            UPDATE clients
            SET last_seen = $1
            WHERE id = $2
            "#,
        )
        .bind(now)
        .bind(client_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    // KeyPackage operations
    async fn store_key_package(&self, key_package: KeyPackage) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO key_packages (id, client_id, data, created_at, used)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(key_package.id)
        .bind(key_package.client_id)
        .bind(key_package.data)
        .bind(key_package.created_at)
        .bind(key_package.used)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_key_package(&self, key_package_id: Uuid) -> DbResult<KeyPackage> {
        let key_package = sqlx::query_as::<_, KeyPackage>(
            r#"
            SELECT * FROM key_packages
            WHERE id = $1
            "#,
        )
        .bind(key_package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?
        .ok_or(DbError::NotFound)?;
        
        Ok(key_package)
    }
    
    async fn list_key_packages_by_client(&self, client_id: Uuid) -> DbResult<Vec<KeyPackage>> {
        let key_packages = sqlx::query_as::<_, KeyPackage>(
            r#"
            SELECT * FROM key_packages
            WHERE client_id = $1 AND used = false
            ORDER BY created_at DESC
            "#,
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(key_packages)
    }
    
    async fn mark_key_package_used(&self, key_package_id: Uuid) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE key_packages
            SET used = true
            WHERE id = $1
            "#,
        )
        .bind(key_package_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    // Group operations
    async fn create_group(&self, group: Group) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO groups (id, creator_id, epoch, state, created_at, updated_at, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(group.id)
        .bind(group.creator_id)
        .bind(group.epoch)
        .bind(group.state)
        .bind(group.created_at)
        .bind(group.updated_at)
        .bind(group.is_active)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_group(&self, group_id: Uuid) -> DbResult<Group> {
        let group = sqlx::query_as::<_, Group>(
            r#"
            SELECT * FROM groups
            WHERE id = $1
            "#,
        )
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?
        .ok_or(DbError::NotFound)?;
        
        Ok(group)
    }
    
    async fn list_groups_by_client(&self, client_id: Uuid) -> DbResult<Vec<Group>> {
        let groups = sqlx::query_as::<_, Group>(
            r#"
            SELECT g.* FROM groups g
            JOIN memberships m ON g.id = m.group_id
            WHERE m.client_id = $1
              AND m.removed_at IS NULL
              AND g.is_active = true
            ORDER BY g.updated_at DESC
            "#,
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(groups)
    }
    
    async fn update_group_epoch(&self, group_id: Uuid, epoch: i64) -> DbResult<()> {
        let now = Utc::now();
        
        sqlx::query(
            r#"
            UPDATE groups
            SET epoch = $1, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(epoch)
        .bind(now)
        .bind(group_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn update_group_state(&self, group_id: Uuid, state: Vec<u8>) -> DbResult<()> {
        let now = Utc::now();
        
        sqlx::query(
            r#"
            UPDATE groups
            SET state = $1, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(state)
        .bind(now)
        .bind(group_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    // Membership operations
    async fn add_membership(&self, membership: Membership) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO memberships (id, client_id, group_id, role, added_at, removed_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(membership.id)
        .bind(membership.client_id)
        .bind(membership.group_id)
        .bind(&membership.role)
        .bind(membership.added_at)
        .bind(membership.removed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn remove_membership(&self, membership_id: Uuid) -> DbResult<()> {
        let now = Utc::now();
        
        sqlx::query(
            r#"
            UPDATE memberships
            SET removed_at = $1
            WHERE id = $2
            "#,
        )
        .bind(now)
        .bind(membership_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn list_memberships_by_group(&self, group_id: Uuid) -> DbResult<Vec<Membership>> {
        let memberships = sqlx::query_as::<_, Membership>(
            r#"
            SELECT * FROM memberships
            WHERE group_id = $1
              AND removed_at IS NULL
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(memberships)
    }
    
    async fn list_memberships_by_client(&self, client_id: Uuid) -> DbResult<Vec<Membership>> {
        let memberships = sqlx::query_as::<_, Membership>(
            r#"
            SELECT * FROM memberships
            WHERE client_id = $1
              AND removed_at IS NULL
            "#,
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(memberships)
    }
    
    // Message operations
    async fn store_message(&self, message: Message) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO messages 
            (id, group_id, sender_id, created_at, read, message_type, 
             proposal, commit, welcome, proposal_type, epoch, recipients)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(message.id)
        .bind(message.group_id)
        .bind(message.sender_id)
        .bind(message.created_at)
        .bind(message.read)
        .bind(&message.message_type)
        .bind(message.proposal)
        .bind(message.commit)
        .bind(message.welcome)
        .bind(message.proposal_type)
        .bind(message.epoch)
        .bind(message.recipients)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn fetch_messages_for_client(&self, client_id: Uuid, group_id: Option<Uuid>, include_read: bool) -> DbResult<Vec<Message>> {
        let query = match (group_id, include_read) {
            (Some(g_id), true) => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.* FROM messages m
                    JOIN memberships mem ON m.group_id = mem.group_id
                    WHERE mem.client_id = $1
                      AND m.group_id = $2
                    ORDER BY m.created_at ASC
                    "#,
                )
                .bind(client_id)
                .bind(g_id)
            },
            (Some(g_id), false) => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.* FROM messages m
                    JOIN memberships mem ON m.group_id = mem.group_id
                    WHERE mem.client_id = $1
                      AND m.group_id = $2
                      AND m.read = false
                    ORDER BY m.created_at ASC
                    "#,
                )
                .bind(client_id)
                .bind(g_id)
            },
            (None, true) => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.* FROM messages m
                    JOIN memberships mem ON m.group_id = mem.group_id
                    WHERE mem.client_id = $1
                    ORDER BY m.created_at ASC
                    "#,
                )
                .bind(client_id)
            },
            (None, false) => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.* FROM messages m
                    JOIN memberships mem ON m.group_id = mem.group_id
                    WHERE mem.client_id = $1
                      AND m.read = false
                    ORDER BY m.created_at ASC
                    "#,
                )
                .bind(client_id)
            },
        };
        
        let messages = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(messages)
    }
    
    async fn mark_messages_read(&self, message_ids: Vec<Uuid>) -> DbResult<()> {
        // Use a transaction to mark all messages as read
        let mut tx = self.pool.begin().await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        for msg_id in &message_ids {
            sqlx::query(
                r#"
                UPDATE messages
                SET read = true
                WHERE id = $1
                "#,
            )
            .bind(msg_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        }
        
        tx.commit().await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        
        Ok(())
    }
} 