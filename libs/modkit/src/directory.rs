//! Directory API - contract for service discovery and instance resolution

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::runtime::{Endpoint, ModuleManager};

/// Information about a service instance
#[derive(Debug, Clone)]
pub struct ServiceInstanceInfo {
    pub module: String,
    pub instance_id: String,
    pub endpoint: Endpoint,
    pub version: Option<String>,
}

/// Information for registering a new module instance
#[derive(Debug, Clone)]
pub struct RegisterInstanceInfo {
    pub module: String,
    pub instance_id: String,
    pub control_endpoint: Option<Endpoint>,
    pub grpc_services: Vec<(String, Endpoint)>,
    pub version: Option<String>,
}

/// Directory API trait for service discovery and instance management
#[async_trait]
pub trait DirectoryApi: Send + Sync {
    /// Resolve a gRPC service by its logical name to an endpoint
    async fn resolve_grpc_service(&self, service_name: &str) -> Result<Endpoint>;

    /// List all service instances for a given module
    async fn list_instances(&self, module: &str) -> Result<Vec<ServiceInstanceInfo>>;

    /// Register a new module instance with the directory
    async fn register_instance(&self, info: RegisterInstanceInfo) -> Result<()>;

    /// Send a heartbeat for a module instance to indicate it's still alive
    async fn send_heartbeat(&self, module: &str, instance_id: &str) -> Result<()>;
}

pub struct LocalDirectoryApi {
    mgr: Arc<ModuleManager>,
}

impl LocalDirectoryApi {
    pub fn new(mgr: Arc<ModuleManager>) -> Self {
        Self { mgr }
    }
}

#[async_trait]
impl DirectoryApi for LocalDirectoryApi {
    async fn resolve_grpc_service(&self, service_name: &str) -> Result<Endpoint> {
        if let Some((_module, _inst, ep)) = self.mgr.pick_service_round_robin(service_name) {
            return Ok(ep);
        }

        anyhow::bail!(
            "Service not found or no healthy instances: {}",
            service_name
        )
    }

    async fn list_instances(&self, module: &str) -> Result<Vec<ServiceInstanceInfo>> {
        let mut result = Vec::new();

        for inst in self.mgr.instances_of(module) {
            if let Some((_, ep)) = inst.grpc_services.iter().next() {
                result.push(ServiceInstanceInfo {
                    module: module.to_string(),
                    instance_id: inst.instance_id.clone(),
                    endpoint: ep.clone(),
                    version: inst.version.clone(),
                });
            }
        }

        Ok(result)
    }

    async fn register_instance(&self, info: RegisterInstanceInfo) -> Result<()> {
        use crate::runtime::ModuleInstance;

        // Build a ModuleInstance from RegisterInstanceInfo
        let mut instance = ModuleInstance::new(info.module.clone(), info.instance_id.clone());

        // Apply control endpoint if provided
        if let Some(control_ep) = info.control_endpoint {
            instance = instance.with_control(control_ep);
        }

        // Apply version if provided
        if let Some(version) = info.version {
            instance = instance.with_version(version);
        }

        // Add all gRPC services
        for (service_name, endpoint) in info.grpc_services {
            instance = instance.with_grpc_service(service_name, endpoint);
        }

        // Register the instance with the manager
        self.mgr.register_instance(Arc::new(instance));

        Ok(())
    }

    async fn send_heartbeat(&self, module: &str, instance_id: &str) -> Result<()> {
        use std::time::Instant;

        self.mgr.update_heartbeat(module, instance_id, Instant::now());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Endpoint, ModuleInstance, ModuleManager};
    use std::sync::Arc;
    use std::time::Instant;

    #[tokio::test]
    async fn test_resolve_rr() {
        let dir = Arc::new(ModuleManager::new());
        let api = LocalDirectoryApi::new(dir.clone());

        // Register two instances providing the same service
        let inst1 = Arc::new(
            ModuleInstance::new("test_module", "instance1")
                .with_grpc_service("test.Service", Endpoint::tcp("127.0.0.1", 8001)),
        );
        let inst2 = Arc::new(
            ModuleInstance::new("test_module", "instance2")
                .with_grpc_service("test.Service", Endpoint::tcp("127.0.0.1", 8002)),
        );

        dir.register_instance(inst1);
        dir.register_instance(inst2);

        // Mark both as healthy
        dir.update_heartbeat("test_module", "instance1", Instant::now());
        dir.update_heartbeat("test_module", "instance2", Instant::now());

        // Resolve should rotate between instances
        let ep1 = api.resolve_grpc_service("test.Service").await.unwrap();
        let ep2 = api.resolve_grpc_service("test.Service").await.unwrap();
        let ep3 = api.resolve_grpc_service("test.Service").await.unwrap();

        // First and third should be the same (round-robin)
        assert_eq!(ep1, ep3);
        // First and second should be different
        assert_ne!(ep1, ep2);
    }

    #[tokio::test]
    async fn test_list_instances_returns_string() {
        let dir = Arc::new(ModuleManager::new());
        let api = LocalDirectoryApi::new(dir.clone());

        let inst = Arc::new(
            ModuleInstance::new("test_module", "instance1")
                .with_grpc_service("test.Service", Endpoint::tcp("127.0.0.1", 8001)),
        );

        dir.register_instance(inst);

        let instances = api.list_instances("test_module").await.unwrap();
        assert_eq!(instances.len(), 1);
        // Verify that module is a String, not &'static str
        assert_eq!(instances[0].module, "test_module".to_string());
    }

    #[tokio::test]
    async fn test_resolve_filters_unhealthy() {
        let dir = Arc::new(ModuleManager::new());
        let api = LocalDirectoryApi::new(dir.clone());

        // Register instance but mark it as quarantined
        let inst = Arc::new(
            ModuleInstance::new("test_module", "instance1")
                .with_grpc_service("test.Service", Endpoint::tcp("127.0.0.1", 8001)),
        );

        dir.register_instance(inst);
        dir.mark_quarantined("test_module", "instance1");

        // Should not resolve quarantined instance
        let result = api.resolve_grpc_service("test.Service").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_instance_via_api() {
        let dir = Arc::new(ModuleManager::new());
        let api = LocalDirectoryApi::new(dir.clone());

        // Register an instance through the API
        let register_info = RegisterInstanceInfo {
            module: "test_module".to_string(),
            instance_id: "instance1".to_string(),
            control_endpoint: Some(Endpoint::tcp("127.0.0.1", 8000)),
            grpc_services: vec![
                ("test.Service".to_string(), Endpoint::tcp("127.0.0.1", 8001)),
            ],
            version: Some("1.0.0".to_string()),
        };

        api.register_instance(register_info).await.unwrap();

        // Verify the instance was registered
        let instances = dir.instances_of("test_module");
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "instance1");
        assert_eq!(instances[0].version, Some("1.0.0".to_string()));
        assert!(instances[0].control.is_some());
        assert!(instances[0].grpc_services.contains_key("test.Service"));
    }

    #[tokio::test]
    async fn test_send_heartbeat_via_api() {
        let dir = Arc::new(ModuleManager::new());
        let api = LocalDirectoryApi::new(dir.clone());

        // Register an instance first
        let inst = Arc::new(ModuleInstance::new("test_module", "instance1"));
        dir.register_instance(inst);

        // Verify initial state is Registered
        let instances = dir.instances_of("test_module");
        assert_eq!(instances[0].state(), crate::runtime::InstanceState::Registered);

        // Send heartbeat via API
        api.send_heartbeat("test_module", "instance1")
            .await
            .unwrap();

        // Verify state transitioned to Healthy
        let instances = dir.instances_of("test_module");
        assert_eq!(instances[0].state(), crate::runtime::InstanceState::Healthy);
    }
}
