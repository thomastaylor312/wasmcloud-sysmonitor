//! Implementation for wasmcloud:messaging
//!
use std::{collections::HashMap, convert::Infallible, sync::Arc};

use sysinfo::{CpuExt, CpuRefreshKind, RefreshKind, System, SystemExt};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn, Instrument};
use uuid::Uuid;
use wasmbus_rpc::{core::LinkDefinition, provider::prelude::*};
use wasmcloud_interface_sysmonitor::{MetricEvent, Sysmonitor, SysmonitorSender, SystemMetrics};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // handle lattice control messages and forward rpc to the provider dispatch
    // returns when provider receives a shutdown control message
    // TODO: Actually load config like timers from the host_data
    let host_data = load_host_data().map_err(|e| {
        eprintln!("error loading host data: {}", &e.to_string());
        Box::new(e)
    })?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let prov = runtime.block_on(SysmonitorBasicProvider::new(Uuid::new_v4().to_string()))?;

    runtime.block_on(provider_run(
        prov,
        host_data,
        Some("SysmonitorBasic Provider".to_string()),
    ))?;
    runtime.shutdown_timeout(core::time::Duration::from_secs(10));

    eprintln!("SysmonitorBasic provider exiting");
    Ok(())
}

#[derive(Clone, Provider)]
struct SysmonitorBasicProvider {
    linked_actors: Arc<RwLock<HashMap<String, LinkDefinition>>>,
}

impl SysmonitorBasicProvider {
    pub async fn new(id: String) -> Result<Self, Box<dyn std::error::Error>> {
        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or_else(|_| {
                eprintln!(
                    "WARN: Could not parse hostname to valid string, defaulting to random UUID"
                );
                Uuid::new_v4().to_string()
            });
        let linked_actors = Arc::new(RwLock::new(HashMap::new()));
        tokio::spawn(run(linked_actors.clone(), id, hostname));
        Ok(Self { linked_actors })
    }
}

// use default implementations of provider message handlers
impl ProviderDispatch for SysmonitorBasicProvider {}

/// Handle provider control commands
#[async_trait]
impl ProviderHandler for SysmonitorBasicProvider {
    /// Provider should perform any operations needed for a new link,
    /// including setting up per-actor resources, and checking authorization.
    /// If the link is allowed, return true, otherwise return false to deny the link.
    #[instrument(level = "info", skip(self))]
    async fn put_link(&self, ld: &LinkDefinition) -> RpcResult<bool> {
        debug!(?ld, "putting link for actor");

        let linkdef = ld.clone();
        self.linked_actors
            .write()
            .await
            .insert(ld.actor_id.clone(), linkdef);

        Ok(true)
    }

    /// Handle notification that a link is dropped: close the connection
    #[instrument(level = "info", skip(self))]
    async fn delete_link(&self, actor_id: &str) {
        debug!(%actor_id, "deleting link for actor");
        self.linked_actors.write().await.remove(actor_id);
    }

    /// Handle shutdown request with any cleanup necessary
    async fn shutdown(&self) -> std::result::Result<(), Infallible> {
        Ok(())
    }
}

#[instrument(level = "info", skip(actors))]
async fn run(actors: Arc<RwLock<HashMap<String, LinkDefinition>>>, id: String, hostname: String) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
    let mut system = System::new_with_specifics(
        RefreshKind::new()
            .with_memory()
            .with_cpu(CpuRefreshKind::everything().without_frequency()),
    );
    loop {
        ticker.tick().await;
        let actors = actors.clone().read_owned().await;
        // Don't try to collect metrics if there are no actors
        if actors.len() == 0 {
            continue;
        }
        system.refresh_all();
        let cpu = system.global_cpu_info();
        let num_cpu = system.cpus().len();
        let system = SystemMetrics {
            num_cpu: Some(num_cpu as u32),
            cpu_usage_percentage: Some(cpu.cpu_usage()),
            memory: Some(system.total_memory()),
            free_memory: Some(system.free_memory()),
            used_memory: Some(system.used_memory()),
            swap: Some(system.total_swap()),
            used_swap: Some(system.used_swap()),
        };
        for actor in actors.values().cloned() {
            let uuid = id.clone();
            let hostname = hostname.clone();
            let span = tracing::debug_span!("publish_metrics", actor_id = %actor.actor_id);
            let cloned_system = system.clone();
            tokio::spawn(
                async move {
                    let client = SysmonitorSender::for_actor(&actor);
                    let event = MetricEvent {
                        uuid,
                        hostname,
                        system: Some(cloned_system),
                        extra_data: None,
                    };
                    if let Err(e) = client
                        .handle_metric_event(&Context::default(), &event)
                        .await
                    {
                        warn!(error = %e, "error publishing metrics")
                    }
                }
                .instrument(span),
            );
        }
    }
}
