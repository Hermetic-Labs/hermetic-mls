#!/bin/bash

# Script to set up the PostgreSQL database for MLS Delivery Service

set -e

# Load environment variables from .env if it exists
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
  echo "Please set the DATABASE_URL environment variable in your .env file"
  echo "Example: DATABASE_URL=postgres://username:password@localhost/mlsdb"
  exit 1
fi

# Extract database name from the URL
DB_NAME=$(echo $DATABASE_URL | sed -e 's/.*\///')

echo "Setting up database: $DB_NAME"

# Check if psql is installed
if ! command -v psql &> /dev/null; then
  echo "psql could not be found. Please install PostgreSQL client tools."
  exit 1
fi

# Run the SQL schema
echo "Applying database schema..."
psql $DATABASE_URL -f schema.sql

echo "Database setup complete!"
echo "You can now run the MLS Delivery Service with: cargo run" 