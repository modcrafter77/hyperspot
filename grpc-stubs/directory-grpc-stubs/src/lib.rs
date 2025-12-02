//! Generated gRPC stubs for DirectoryService
//!
//! This crate contains only the generated protobuf types and gRPC client/server stubs
//! for the DirectoryService. It does not contain any business logic.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

/// Generated protobuf types for DirectoryService
pub mod pb {
    tonic::include_proto!("modkit.directory.v1");
}

// Re-export common types for convenience
pub use pb::directory_service_client::DirectoryServiceClient;
pub use pb::directory_service_server::{DirectoryService, DirectoryServiceServer};
pub use pb::{
    HeartbeatRequest, InstanceInfo, ListInstancesRequest, ListInstancesResponse,
    RegisterInstanceRequest, ResolveGrpcServiceRequest, ResolveGrpcServiceResponse,
};

