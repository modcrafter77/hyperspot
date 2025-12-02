//! Out-of-process module bootstrap library
//!
//! This library provides reusable functionality for bootstrapping OoP (out-of-process)
//! modkit modules in local (non-k8s) environments.
//!
//! ## Features
//!
//! - Configuration loading using `modkit-bootstrap`
//! - Logging initialization with tracing
//! - gRPC connection to DirectoryService
//! - Module instance registration
//! - Heartbeat management
//! - Module lifecycle execution
//!
//! ## Example
//!
//! ```rust,no_run
//! use modkit_oop_bootstrap::{OopRunOptions, run_oop_with_options};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let opts = OopRunOptions {
//!         module_name: "my_module".to_string(),
//!         instance_id: None,
//!         directory_endpoint: "http://127.0.0.1:50051".to_string(),
//!         config_path: None,
//!         verbose: 0,
//!         print_config: false,
//!         heartbeat_interval_secs: 5,
//!     };
//!     
//!     run_oop_with_options(opts).await
//! }
//! ```

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

use modkit::runtime::{run, DbOptions, RunOptions, ShutdownOptions};
use modkit::{DirectoryApi, RegisterInstanceInfo};
use modkit_bootstrap::{AppConfig, AppConfigProvider, CliArgs, ConfigProvider as _};

/// Configuration options for OoP module bootstrap
#[derive(Debug, Clone)]
pub struct OopRunOptions {
    /// Logical module name (e.g., "file_parser")
    pub module_name: String,

    /// Instance ID (defaults to a random UUID if None)
    pub instance_id: Option<String>,

    /// Directory service gRPC endpoint (e.g., "http://127.0.0.1:50051")
    pub directory_endpoint: String,

    /// Path to configuration file
    pub config_path: Option<PathBuf>,

    /// Log verbosity level (0=default, 1=info, 2=debug, 3=trace)
    pub verbose: u8,

    /// Print effective configuration and exit
    pub print_config: bool,

    /// Heartbeat interval in seconds (default: 5)
    pub heartbeat_interval_secs: u64,
}

impl Default for OopRunOptions {
    fn default() -> Self {
        Self {
            module_name: String::new(),
            instance_id: None,
            directory_endpoint: "http://127.0.0.1:50051".to_string(),
            config_path: None,
            verbose: 0,
            print_config: false,
            heartbeat_interval_secs: 5,
        }
    }
}

/// Adapter to make `AppConfigProvider` implement `modkit::ConfigProvider`
struct ModkitConfigAdapter(Arc<AppConfigProvider>);

impl modkit::ConfigProvider for ModkitConfigAdapter {
    fn get_module_config(&self, module_name: &str) -> Option<&serde_json::Value> {
        self.0.get_module_config(module_name)
    }
}

/// Run an out-of-process module with the given options
///
/// This function:
/// 1. Loads configuration and initializes logging
/// 2. Connects to the DirectoryService
/// 3. Registers the module instance
/// 4. Starts a background heartbeat loop
/// 5. Runs the module lifecycle (init, start, etc.)
///
/// # Arguments
///
/// * `opts` - Bootstrap configuration options
///
/// # Returns
///
/// * `Ok(())` - If the module lifecycle completed successfully
/// * `Err(e)` - If any step failed
///
/// # Example
///
/// ```rust,no_run
/// use modkit_oop_bootstrap::{OopRunOptions, run_oop_with_options};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let opts = OopRunOptions {
///         module_name: "file_parser".to_string(),
///         instance_id: None,
///         directory_endpoint: "http://127.0.0.1:50051".to_string(),
///         config_path: None,
///         verbose: 1,
///         print_config: false,
///         heartbeat_interval_secs: 5,
///     };
///
///     run_oop_with_options(opts).await
/// }
/// ```
pub async fn run_oop_with_options(opts: OopRunOptions) -> Result<()> {
    // Generate instance ID if not provided
    let instance_id = opts
        .instance_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Prepare CLI args for AppConfig loading
    let args = CliArgs {
        config: opts.config_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        print_config: opts.print_config,
        verbose: opts.verbose,
        mock: false,
    };

    // Load configuration
    let mut config = AppConfig::load_or_default(opts.config_path.as_deref())?;
    config.apply_cli_overrides(&args);

    // Initialize logging (without OTEL for now, can be extended)
    let logging_config = config.logging.as_ref().cloned().unwrap_or_default();
    modkit_bootstrap::logging::init_logging_unified(
        &logging_config,
        std::path::Path::new(&config.server.home_dir),
        None, // No OTEL layer for now
    );

    info!(
        module = %opts.module_name,
        instance_id = %instance_id,
        directory_endpoint = %opts.directory_endpoint,
        "OoP module bootstrap starting"
    );

    // Print config and exit if requested
    if opts.print_config {
        println!("{}", config.to_yaml()?);
        return Ok(());
    }

    // Connect to DirectoryService
    info!("Connecting to directory service at {}", opts.directory_endpoint);
    let directory_client = connect_directory(&opts.directory_endpoint).await?;
    let directory_api: Arc<dyn DirectoryApi> = Arc::new(directory_client);

    info!("Successfully connected to directory service");

    // Register this instance with the directory
    info!("Registering module instance");
    let register_info = RegisterInstanceInfo {
        module: opts.module_name.clone(),
        instance_id: instance_id.clone(),
        control_endpoint: None, // Can be extended later
        grpc_services: vec![],  // Will be populated when module starts its gRPC services
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };

    directory_api.register_instance(register_info).await?;
    info!("Module instance registered successfully");

    // Start heartbeat loop in background
    let heartbeat_directory = Arc::clone(&directory_api);
    let heartbeat_module = opts.module_name.clone();
    let heartbeat_instance_id = instance_id.clone();
    let heartbeat_interval = Duration::from_secs(opts.heartbeat_interval_secs);

    tokio::spawn(async move {
        info!(
            interval_secs = opts.heartbeat_interval_secs,
            "Starting heartbeat loop"
        );

        loop {
            sleep(heartbeat_interval).await;

            match heartbeat_directory
                .send_heartbeat(&heartbeat_module, &heartbeat_instance_id)
                .await
            {
                Ok(_) => {
                    tracing::debug!("Heartbeat sent successfully");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to send heartbeat, will retry");
                }
            }
        }
    });

    // Build config provider for modules
    let config_provider = Arc::new(ModkitConfigAdapter(Arc::new(AppConfigProvider::new(
        config.clone(),
    ))));

    // For OoP modules, we typically don't need databases initially
    // This can be extended later if needed
    let db_options = DbOptions::None;

    // Run the module lifecycle
    info!("Starting module lifecycle");
    let run_options = RunOptions {
        modules_cfg: config_provider,
        db: db_options,
        shutdown: ShutdownOptions::Signals,
    };

    let result = run(run_options).await;

    if let Err(ref e) = result {
        error!(error = %e, "Module runtime failed");
    } else {
        info!("Module runtime completed successfully");
    }

    result
}

/// Connect to DirectoryService via gRPC
///
/// This is a helper function that creates a DirectoryService gRPC client
/// using the generated stubs.
async fn connect_directory(endpoint: &str) -> Result<DirectoryGrpcClient> {
    use directory_grpc_stubs::DirectoryServiceClient;
    use modkit_transport_grpc::client::GrpcClientConfig;

    let cfg = GrpcClientConfig::new("directory_service");

    // Create endpoint with timeouts from config
    let endpoint_obj = tonic::transport::Endpoint::from_shared(endpoint.to_string())?
        .connect_timeout(cfg.connect_timeout)
        .timeout(cfg.rpc_timeout);

    // Connect to the service
    let channel = endpoint_obj.connect().await?;

    if cfg.enable_tracing {
        tracing::debug!(
            service_name = cfg.service_name,
            connect_timeout_ms = cfg.connect_timeout.as_millis(),
            rpc_timeout_ms = cfg.rpc_timeout.as_millis(),
            "directory gRPC client connected"
        );
    }

    Ok(DirectoryGrpcClient {
        inner: DirectoryServiceClient::new(channel),
    })
}

/// gRPC client implementation of DirectoryApi
pub struct DirectoryGrpcClient {
    inner: directory_grpc_stubs::DirectoryServiceClient<tonic::transport::Channel>,
}

#[async_trait::async_trait]
impl DirectoryApi for DirectoryGrpcClient {
    async fn resolve_grpc_service(&self, service_name: &str) -> Result<modkit::runtime::Endpoint> {
        use directory_grpc_stubs::ResolveGrpcServiceRequest;

        let mut client = self.inner.clone();
        let request = tonic::Request::new(ResolveGrpcServiceRequest {
            service_name: service_name.to_string(),
        });

        let response = client
            .resolve_grpc_service(request)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC call failed: {}", e))?;

        let proto_response = response.into_inner();
        Ok(modkit::runtime::Endpoint {
            uri: proto_response.endpoint_uri,
        })
    }

    async fn list_instances(
        &self,
        module: &str,
    ) -> Result<Vec<modkit::ServiceInstanceInfo>> {
        use directory_grpc_stubs::ListInstancesRequest;

        let mut client = self.inner.clone();
        let request = tonic::Request::new(ListInstancesRequest {
            module_name: module.to_string(),
        });

        let response = client
            .list_instances(request)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC call failed: {}", e))?;

        let proto_response = response.into_inner();

        // Convert proto instances to domain types
        let instances = proto_response
            .instances
            .into_iter()
            .map(|proto_inst| modkit::ServiceInstanceInfo {
                module: proto_inst.module_name,
                instance_id: proto_inst.instance_id,
                endpoint: modkit::runtime::Endpoint {
                    uri: proto_inst.endpoint_uri,
                },
                version: if proto_inst.version.is_empty() {
                    None
                } else {
                    Some(proto_inst.version)
                },
            })
            .collect();

        Ok(instances)
    }

    async fn register_instance(&self, info: RegisterInstanceInfo) -> Result<()> {
        use directory_grpc_stubs::RegisterInstanceRequest;

        let mut client = self.inner.clone();

        let control = info
            .control_endpoint
            .as_ref()
            .map(|e| e.uri.clone())
            .unwrap_or_default();

        // For now, pass only service names. If needed later, extend proto with per-service endpoints.
        let grpc_services = info
            .grpc_services
            .into_iter()
            .map(|(name, _ep)| name)
            .collect();

        let req = RegisterInstanceRequest {
            module_name: info.module,
            instance_id: info.instance_id,
            control_endpoint: control,
            grpc_services,
            version: info.version.unwrap_or_default(),
        };

        client
            .register_instance(tonic::Request::new(req))
            .await
            .map_err(|e| anyhow::anyhow!("gRPC register_instance failed: {}", e))?;

        Ok(())
    }

    async fn send_heartbeat(&self, module: &str, instance_id: &str) -> Result<()> {
        use directory_grpc_stubs::HeartbeatRequest;

        let mut client = self.inner.clone();

        let req = HeartbeatRequest {
            module_name: module.to_string(),
            instance_id: instance_id.to_string(),
        };

        client
            .heartbeat(tonic::Request::new(req))
            .await
            .map_err(|e| anyhow::anyhow!("gRPC heartbeat failed: {}", e))?;

        Ok(())
    }
}

