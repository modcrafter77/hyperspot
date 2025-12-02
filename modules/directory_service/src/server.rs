//! gRPC server implementation for DirectoryService

use std::sync::Arc;
use tonic::{Request, Response, Status};

use modkit::{DirectoryApi, RegisterInstanceInfo};
use modkit::runtime::Endpoint;

// Import from grpc-stubs
use directory_grpc_stubs::{
    DirectoryService, DirectoryServiceServer,
    HeartbeatRequest, InstanceInfo, ListInstancesRequest, ListInstancesResponse,
    RegisterInstanceRequest, ResolveGrpcServiceRequest, ResolveGrpcServiceResponse,
};

// Export the service name constant for use by the module
pub const SERVICE_NAME: &str = directory_grpc_stubs::pb::directory_service_server::SERVICE_NAME;

/// gRPC service implementation that wraps DirectoryApi
#[derive(Clone)]
pub struct DirectoryServiceImpl {
    api: Arc<dyn DirectoryApi>,
}

impl DirectoryServiceImpl {
    pub fn new(api: Arc<dyn DirectoryApi>) -> Self {
        Self { api }
    }
}

#[tonic::async_trait]
impl DirectoryService for DirectoryServiceImpl {
    async fn resolve_grpc_service(
        &self,
        request: Request<ResolveGrpcServiceRequest>,
    ) -> Result<Response<ResolveGrpcServiceResponse>, Status> {
        let service_name = request.into_inner().service_name;

        let endpoint = self
            .api
            .resolve_grpc_service(&service_name)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;

        Ok(Response::new(ResolveGrpcServiceResponse {
            endpoint_uri: endpoint.uri,
        }))
    }

    async fn list_instances(
        &self,
        request: Request<ListInstancesRequest>,
    ) -> Result<Response<ListInstancesResponse>, Status> {
        let module_name = request.into_inner().module_name;

        let instances = self
            .api
            .list_instances(&module_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let resp = ListInstancesResponse {
            instances: instances
                .into_iter()
                .map(|i| InstanceInfo {
                    module_name: i.module,
                    instance_id: i.instance_id,
                    endpoint_uri: i.endpoint.uri,
                    version: i.version.unwrap_or_default(),
                })
                .collect(),
        };

        Ok(Response::new(resp))
    }

    async fn register_instance(
        &self,
        request: Request<RegisterInstanceRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        // Convert RegisterInstanceRequest to RegisterInstanceInfo
        let control_endpoint = if req.control_endpoint.is_empty() {
            None
        } else {
            Some(Endpoint::from_uri(req.control_endpoint))
        };

        // For now, we only have service names in the proto.
        // In the future, we could extend the proto to include per-service endpoints.
        // For now, we'll create dummy endpoints or skip this field.
        let grpc_services = req
            .grpc_services
            .into_iter()
            .map(|name| {
                // Use a placeholder endpoint since we don't have individual service endpoints
                // in the proto yet. This can be extended later if needed.
                (name.clone(), Endpoint::from_uri(format!("placeholder://{}", name)))
            })
            .collect();

        let info = RegisterInstanceInfo {
            module: req.module_name,
            instance_id: req.instance_id,
            control_endpoint,
            grpc_services,
            version: if req.version.is_empty() {
                None
            } else {
                Some(req.version)
            },
        };

        self.api
            .register_instance(info)
            .await
            .map_err(|e| Status::internal(format!("Failed to register instance: {}", e)))?;

        Ok(Response::new(()))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        self.api
            .send_heartbeat(&req.module_name, &req.instance_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to send heartbeat: {}", e)))?;

        Ok(Response::new(()))
    }
}

/// Create a DirectoryService server with the given API implementation
pub fn make_directory_service(
    api: Arc<dyn DirectoryApi>,
) -> DirectoryServiceServer<DirectoryServiceImpl> {
    DirectoryServiceServer::new(DirectoryServiceImpl::new(api))
}
