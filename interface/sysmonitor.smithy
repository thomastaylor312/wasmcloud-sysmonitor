// sysmonitor.smithy
// A service that can notify the current metrics of a system

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "org.wasmcloud.interface.experimental.sysmonitor", crate: "wasmcloud_interface_sysmonitor" } ]

namespace org.wasmcloud.interface.experimental.sysmonitor

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#codegenRust
use org.wasmcloud.model#U32
use org.wasmcloud.model#U64
use org.wasmcloud.model#F32

/// The Sysmonitor service has a single method for actor receivers to implement. The received event
/// is not guaranteed to arrive on any specific schedule 
@wasmbus(
    contractId: "wasmcloud:exp:sysmonitor",
    actorReceive: true)
service Sysmonitor {
  version: "0.1",
  operations: [ HandleMetricEvent ]
}

/// Sends a MetricEvent to an actor to be handled
operation HandleMetricEvent {
  input: MetricEvent,
}

/// An event containing observed metrics
@codegenRust( noDeriveEq: true, noDeriveDefault: true )
structure MetricEvent {
  /// The hostname where these metrics are coming from
  @required
  hostname: String,

  /// A unique UUIDv4 for the provider sending this data. This can be generated on provider start
  @required
  uuid: String,

  /// Metrics generated from the system information
  system: SystemMetrics,

  // TODO: Eventually add more built in ones with maybe things like host information

  /// Extra statistics that can be sent. Must be encoded as a map of string, string
  extra_data: StringMap,
}

map StringMap {
  key: String,
  value: String,
}

/// Metrics from the system. All of these metrics could be entirely optional depending on what
/// system is being observed
@codegenRust( noDeriveEq: true )
structure SystemMetrics {
  /// The number of CPUs available to the system (if applicable)
  num_cpu: U32,

  /// Percentage of CPU used by user space processes (if applicable)
  cpu_usage_percentage: F32,

  /// The amount of memory for the system (if applicable)
  memory: U64,

  /// The amount of memory used (if applicable)
  used_memory: U64,

  /// The amount of memory free (if applicable)
  free_memory: U64,

  /// The amount of swap for the system (if applicable)
  swap: U64,

  /// The amount of swap used (if applicable)
  used_swap: U64,

  /// The amount of swap free (if applicable)
  free_memory: U64,
}

