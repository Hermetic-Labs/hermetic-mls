![hermetic mls cover](./assets/cover.png)

# hermetic-mls 🔮: gRPC OpenMLS Delivery Service

A gRPC-based delivery service for MLS messaging that will eventually handle all core MLS functionality using OpenMLS and persists state to PostgreSQL using SQLx.

## Overview

This delivery service provides:

1. Client registration and identity management
2. KeyPackage publication and retrieval
3. MLS group creation and management
4. Secure storage of MLS messages (proposals, commits, welcome)
5. Membership tracking for groups

## Requirements

- Rust 1.70+ and Cargo
- PostgreSQL 12+

## PostgreSQL Setup

Create the following tables in your PostgreSQL database:

### Groups
```sql
CREATE TABLE groups (
  id UUID PRIMARY KEY,
  creator_id UUID NOT NULL,
  epoch BIGINT NOT NULL DEFAULT 0,
  state BYTEA,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  is_active BOOLEAN NOT NULL DEFAULT true
);
```

### Clients
```sql
CREATE TABLE clients (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  credential BYTEA NOT NULL,
  scheme TEXT NOT NULL,
  device_name TEXT NOT NULL,
  last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### Memberships
```sql
CREATE TABLE memberships (
  id UUID PRIMARY KEY,
  client_id UUID NOT NULL REFERENCES clients(id),
  group_id UUID NOT NULL REFERENCES groups(id),
  role TEXT NOT NULL,
  added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  removed_at TIMESTAMPTZ
);
```

### Messages
```sql
CREATE TABLE messages (
  id UUID PRIMARY KEY,
  group_id UUID NOT NULL REFERENCES groups(id),
  sender_id UUID NOT NULL REFERENCES clients(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  read BOOLEAN NOT NULL DEFAULT false,
  message_type TEXT NOT NULL,
  proposal BYTEA,
  commit BYTEA,
  welcome BYTEA,
  proposal_type TEXT,
  epoch BIGINT,
  recipients UUID[] 
);
```

### KeyPackages
```sql
CREATE TABLE key_packages (
  id UUID PRIMARY KEY,
  client_id UUID NOT NULL REFERENCES clients(id),
  data BYTEA NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  used BOOLEAN NOT NULL DEFAULT false
);
```

## Configuration

Create a `.env` file with the following configuration:

```
# PostgreSQL configuration
DATABASE_URL=postgres://username:password@localhost/mlsdb

# Logging level (debug, info, warn, error)
RUST_LOG=info

# Address to bind the server to
ADDR=0.0.0.0:50051
```

## Building and Running

```bash
# Build the project
cargo build --release

# Run the service
cargo run --release
```

## gRPC API

The service exposes the following gRPC endpoints:

### Client Operations
- `RegisterClient`: Register a new client with credential
- `GetClient`: Retrieve client information
- `ListClients`: List all clients for a user

### KeyPackage Operations
- `PublishKeyPackage`: Publish a key package for a client
- `GetKeyPackage`: Retrieve a specific key package
- `ListKeyPackages`: List all key packages for a client

### Group Operations
- `CreateGroup`: Create a new MLS group
- `GetGroup`: Retrieve group information
- `ListGroups`: List all groups a client is a member of

### Membership Operations
- `AddMember`: Add a client to a group
- `RemoveMember`: Remove a client from a group
- `ListMemberships`: List all memberships for a group

### MLS Message Operations
- `StoreProposal`: Store an MLS proposal message
- `StoreCommit`: Store an MLS commit message
- `StoreWelcome`: Store an MLS welcome message
- `FetchMessages`: Fetch messages for a client

## Database Connection

This service uses SQLx to connect to PostgreSQL. SQLx is:

1. An async, pure Rust SQL crate featuring compile-time checked queries
2. Supporting PostgreSQL, MySQL, SQLite, and MSSQL
3. Fully asynchronous using Tokio for database operations

The application follows a repository pattern with a clean separation between:

- Database interface definition (`DatabaseInterface` trait)
- Implementation for PostgreSQL (`PostgresDatabase`)
- Service layer implementing the gRPC methods

## Security Considerations

1. All MLS cryptographic operations are handled by the OpenMLS library
2. Messages are stored in encrypted form as provided by the clients
3. Always use a secure, limited-permission database user in production

## License

See the LICENSE file for details.

## Testing

The project includes comprehensive integration tests to verify the functionality of the MLS Delivery Service. The tests use a mock database implementation to avoid external dependencies.

To run the tests, use the following command:

```bash
cargo test
```

The integration tests cover:

- Client registration and lifecycle
- Key package publishing and management
- Group creation and management
- Message delivery (proposals, commits, and welcome messages)
- Error handling

### Test Structure

- `tests/common.rs` - Common utilities and mock database implementation
- `tests/integration_tests.rs` - Basic service functionality tests
- `tests/client_lifecycle_tests.rs` - Tests for client registration and management
- `tests/welcome_test.rs` - Tests for welcome message handling
- `tests/error_handling_tests.rs` - Tests for error conditions and edge cases 
