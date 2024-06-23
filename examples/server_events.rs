use std::sync::atomic::{AtomicI64, Ordering};

use valkey_module_macros::{config_changed_event_handler, cron_event_handler, flush_event_handler};
use valkey_module::{
    server_events::FlushSubevent, valkey_module, Context, ValkeyResult, ValkeyString, ValkeyValue,
};

static NUM_FLUSHES: AtomicI64 = AtomicI64::new(0);
static NUM_CRONS: AtomicI64 = AtomicI64::new(0);
static NUM_MAX_MEMORY_CONFIGURATION_CHANGES: AtomicI64 = AtomicI64::new(0);

#[flush_event_handler]
fn flushed_event_handler(_ctx: &Context, flush_event: FlushSubevent) {
    if let FlushSubevent::Started = flush_event {
        NUM_FLUSHES.fetch_add(1, Ordering::SeqCst);
    }
}

#[config_changed_event_handler]
fn config_changed_event_handler(_ctx: &Context, changed_configs: &[&str]) {
    changed_configs
        .iter()
        .find(|v| **v == "maxmemory")
        .map(|_| NUM_MAX_MEMORY_CONFIGURATION_CHANGES.fetch_add(1, Ordering::SeqCst));
}

#[cron_event_handler]
fn cron_event_handler(_ctx: &Context, _hz: u64) {
    NUM_CRONS.fetch_add(1, Ordering::SeqCst);
}

fn num_flushed(_ctx: &Context, _args: Vec<ValkeyString>) -> ValkeyResult {
    Ok(ValkeyValue::Integer(NUM_FLUSHES.load(Ordering::SeqCst)))
}

fn num_crons(_ctx: &Context, _args: Vec<ValkeyString>) -> ValkeyResult {
    Ok(ValkeyValue::Integer(NUM_CRONS.load(Ordering::SeqCst)))
}

fn num_maxmemory_changes(_ctx: &Context, _args: Vec<ValkeyString>) -> ValkeyResult {
    Ok(ValkeyValue::Integer(
        NUM_MAX_MEMORY_CONFIGURATION_CHANGES.load(Ordering::SeqCst),
    ))
}

//////////////////////////////////////////////////////

valkey_module! {
    name: "srv_events",
    version: 1,
    allocator: (valkey_module::alloc::ValkeyAlloc, valkey_module::alloc::ValkeyAlloc),
    data_types: [],
    commands: [
        ["num_flushed", num_flushed, "readonly", 0, 0, 0],
        ["num_max_memory_changes", num_maxmemory_changes, "readonly", 0, 0, 0],
        ["num_crons", num_crons, "readonly", 0, 0, 0],
    ],
}
