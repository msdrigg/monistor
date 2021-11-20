use std::{collections::HashMap, sync::Arc};

use dbus::{
    arg::PropMap,
    nonblock::{Proxy, SyncConnection},
};

use crate::{codegen::MutterDisplayConfig, interface::*};
/// Updates the monitor configuration by aliginging the
/// monitors provided by `DEFAULT_MONITORS` horizontally
/// from left to right. With their top's aligned.
///
/// If any monitors can't be found, then skip them
///
/// Make the leftmost monitor primary
/// # Algorithm
/// 1. Collect all target monitors into a map from id to top right corner position
/// 2. Check if these monitors are already contained in a logical monitor with the correct corner position
/// 3. Verify that the primary monitor is the left monitor (and update it if it is not)
/// 4. Choose largest width resolution if multiple are available
pub async fn update_monitors(
    monitor_array: &[&str],
    proxy: &Proxy<'_, Arc<SyncConnection>>,
) -> Result<(), anyhow::Error> {
    log::info!("Calling GetCurrentState");
    let (serial, monitors, logical_monitors, properties): (
        u32,
        Vec<MonitorRaw>,
        Vec<LogicalMonitorRaw>,
        PropMap,
    ) = proxy.get_current_state().await?;

    log::trace!("Gotten resources: \nserial: {},\nlogical_monitors: {:#?},\nhardware_monitors: {:#?},\nmodes: {:#?}", serial, monitors, logical_monitors, properties);
    let logical_monitor_vec: Vec<LogicalMonitor> = logical_monitors
        .into_iter()
        .map(|val: LogicalMonitorRaw| val.into())
        .collect();
    // Assuming that connector is unique
    let monitor_map: HashMap<_, Monitor> = monitors
        .into_iter()
        .map(|val: MonitorRaw| (val.0 .0.clone(), val.into()))
        .collect();

    let target_monitors: Vec<&Monitor> = monitor_array
        .iter()
        .filter_map(|&x| monitor_map.get(x))
        .collect();

    if target_monitors.len() == 0 {
        log::info!("No monitors matched our target monitors, returning");
        return Ok(());
    } else {
        log::trace!("Using monitors {:#?}", target_monitors);
    }

    let first_monitor = target_monitors.first().map(|&val| &val.info.connector);
    let requested_monitor_updates: Vec<LogicalMonitorUpdate> = target_monitors
        .into_iter()
        .scan(0_i32, |pos: &mut i32, monitor| {
            let output_mode = monitor
                .modes
                .iter()
                .max_by(|&mode_a, &mode_b| {
                    let initial_comparison = mode_a
                        .width
                        .cmp(&mode_b.width)
                        .then(mode_a.height.cmp(&mode_b.height));
                    match mode_a.refresh_rate.partial_cmp(&mode_b.refresh_rate) {
                        Some(cmp) => initial_comparison.then(cmp),
                        None => initial_comparison,
                    }
                })
                .unwrap();

            let should_be_primary = Some(&monitor.info.connector) == first_monitor;

            let previous_pos_x = pos.clone();
            *pos += output_mode.width as i32;

            Some(LogicalMonitorUpdate {
                x: previous_pos_x,
                y: 0,
                scale: output_mode.preferred_scale,
                transform: Transform::Normal,
                is_primary: should_be_primary,
                monitors: vec![MonitorUpdateInfo {
                    connector: monitor.info.connector.to_string(),
                    mode: output_mode.id.to_string(),
                    properties: HashMap::new(),
                }],
            })
        })
        .collect();

    log::trace!("Expecting monitor updates {:#?}", requested_monitor_updates);

    let monitor_vec = &monitor_map.into_iter().map(|(_, v)| v).collect();
    if !requested_monitor_updates.iter().all(|update| {
        logical_monitor_vec
            .iter()
            .any(|existing_monitor| update.matches_existing_state(existing_monitor, monitor_vec))
    }) {
        log::info!("Updating monitors with monitor_changes");
        let logical_monitor_updates: Vec<LogicalMonitorUpdateRaw> = requested_monitor_updates
            .into_iter()
            .map(|monitor| monitor.into())
            .collect();
        proxy
            .apply_monitors_config(
                serial,
                Method::Persistent as u32,
                logical_monitor_updates,
                HashMap::new(),
            )
            .await
            .map_err(|err| {
                log::error!("Error calling ApplyConfiguration, \n{:#?}", err);
                err
            })?;
        log::info!("Update succeeded");
    } else {
        log::info!("Nothing to change. Monitors are working");
    }

    Ok(())
}
