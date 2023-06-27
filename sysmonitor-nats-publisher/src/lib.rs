use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_messaging::{Messaging, MessagingSender, PubMessage};
use wasmcloud_interface_sysmonitor::{MetricEvent, Sysmonitor, SysmonitorReceiver};

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, Sysmonitor)]
struct SysmonitorNatsPublisherActor {}

#[async_trait]
impl Sysmonitor for SysmonitorNatsPublisherActor {
    /// handle subscription response
    async fn handle_metric_event(&self, ctx: &Context, msg: &MetricEvent) -> RpcResult<()> {
        let data = serde_json::to_vec(msg).map_err(|e| RpcError::Ser(e.to_string()))?;
        MessagingSender::new()
            .publish(
                ctx,
                &PubMessage {
                    subject: "wasmcloud.sysmonitor".to_string(),
                    reply_to: None,
                    body: data,
                },
            )
            .await?;
        Ok(())
    }
}
