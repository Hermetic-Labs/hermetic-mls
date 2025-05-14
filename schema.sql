-- MLS Delivery Service Database Schema
-- This schema is used to store the data for the MLS Delivery Service
-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table: This table is a flexible table that can be used to store any user data as long as the id is tied to a client
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY,
  display_name TEXT,
  username TEXT,
  email TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  is_active BOOLEAN NOT NULL DEFAULT true
);

-- Groups table: This table is used to store the groups that are created by the users
CREATE TABLE IF NOT EXISTS groups (
  id UUID PRIMARY KEY,
  creator_id UUID NOT NULL,
  epoch BIGINT NOT NULL DEFAULT 0,
  state BYTEA,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  is_active BOOLEAN NOT NULL DEFAULT true
);

-- Clients table: This table is used to store the clients that are created by the users
CREATE TABLE IF NOT EXISTS clients (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  credential BYTEA NOT NULL,
  scheme TEXT NOT NULL,
  device_name TEXT NOT NULL,
  last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Key packages table: This table is used to store the key packages that are created by the clients
CREATE TABLE IF NOT EXISTS key_packages (
  id UUID PRIMARY KEY,
  client_id UUID NOT NULL REFERENCES clients(id),
  data BYTEA NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  used BOOLEAN NOT NULL DEFAULT false
);

-- Memberships table: This table is used to store the memberships that are created by the clients
CREATE TABLE IF NOT EXISTS memberships (
  id UUID PRIMARY KEY,
  client_id UUID NOT NULL REFERENCES clients(id),
  group_id UUID NOT NULL REFERENCES groups(id),
  role TEXT NOT NULL,
  added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  removed_at TIMESTAMPTZ
);

-- Messages table: This table is used to store the messages that are sent by the clients
CREATE TABLE IF NOT EXISTS messages (
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

-- Indexes for better performance
CREATE INDEX IF NOT EXISTS idx_clients_user_id ON clients(user_id);
CREATE INDEX IF NOT EXISTS idx_key_packages_client_id ON key_packages(client_id);
CREATE INDEX IF NOT EXISTS idx_memberships_group_id ON memberships(group_id);
CREATE INDEX IF NOT EXISTS idx_memberships_client_id ON memberships(client_id);
CREATE INDEX IF NOT EXISTS idx_messages_group_id ON messages(group_id);
CREATE INDEX IF NOT EXISTS idx_messages_sender_id ON messages(sender_id); 