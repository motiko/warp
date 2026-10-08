/// Telemetry is never sent; see [`warp_core::send_telemetry_from_ctx`].
#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr_2021, $ctx:expr_2021) => {{
        let _ = $event;
        let _ = &$ctx;
    }};
}

/// Telemetry is never sent; see [`warp_core::send_telemetry_from_ctx`].
#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr_2021, $ctx:expr_2021) => {{
        let _ = $event;
        let _ = &$ctx;
    }};
}

/// Telemetry is never sent; see [`warp_core::send_telemetry_from_ctx`].
#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($event:expr_2021, $executor:expr_2021) => {{
        let _ = $event;
        let _ = &$executor;
    }};
}
