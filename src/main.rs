mod db;
mod service;

use std::error::Error;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use dotenv::dotenv;
use log::info;
use pretty_env_logger;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;
use tower_http::cors::{Any, CorsLayer};
use tonic_reflection::server::Builder as ReflectionBuilder;

use crate::service::mls::mls_delivery_service_server::MlsDeliveryServiceServer;
use crate::service::MLSServiceImpl;
use crate::service::mls;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    info!("Starting MLS Delivery Service");

    // Load environment variables from .env file if present
    dotenv().ok();
    
    // Get configuration from environment variables
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()
        .expect("Invalid address format in ADDR environment variable");

    // Get required database connection string
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable is required");
    
    // Set up connection pool with PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Could not connect to database");
    
    // Initialize the database interface
    let db = Arc::new(db::PostgresDatabase::new(pool));
    
    // Run migrations
    info!("Running database migrations");
    db.migrate_clients_table().await.expect("Failed to migrate clients table");
    
    // Create the MLS service implementation
    let mls_service = MLSServiceImpl::new(db);
    
    // Create a CORS layer that allows any origin
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Setup the gRPC server with reflection
    info!("Starting MLS Delivery Service on {}", addr);

    // Create the reflection service using your file descriptor set
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(mls::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    Server::builder()
        .layer(cors)
        .add_service(reflection_service)
        .add_service(MlsDeliveryServiceServer::new(mls_service))
        .serve(addr)
        .await?;
    
    Ok(())
} 