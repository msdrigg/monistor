use std::convert::TryFrom;

use dbus::arg::{PropMap, prop_cast};

pub(crate) type LogicalMonitorRaw = (
    i32,
    i32,
    f64,
    u32,
    bool,
    Vec<(String, String, String, String)>,
    PropMap,
);
#[derive(Debug)]
pub struct LogicalMonitor {
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: Transform,
    pub is_primary: bool,
    pub monitors: Vec<MonitorInfo>,
    pub properties: PropMap,
}

impl From<LogicalMonitorRaw> for LogicalMonitor {
    fn from(val: LogicalMonitorRaw) -> Self {
        LogicalMonitor {
            x: val.0,
            y: val.1,
            scale: val.2,
            transform: Transform::try_from(val.3).unwrap(),
            is_primary: val.4,
            monitors: val
                .5
                .into_iter()
                .map(|info| MonitorInfo {
                    connector: info.0,
                    vendor: info.1,
                    product_name: info.2,
                    serial: info.3,
                })
                .collect(),
            properties: val.6,
        }
    }
}
impl From<LogicalMonitor> for LogicalMonitorRaw {
    fn from(val: LogicalMonitor) -> Self {
        (
            val.x,
            val.y,
            val.scale,
            val.transform as u32,
            val.is_primary,
            val.monitors
                .into_iter()
                .map(|info| (info.connector, info.vendor, info.product_name, info.serial))
                .collect(),
            val.properties,
        )
    }
}

pub(crate) type MonitorRaw = ((String, String, String, String), Vec<ModeRaw>, PropMap);

#[derive(Debug)]
pub struct MonitorInfo {
    pub connector: String,
    pub vendor: String,
    pub product_name: String,
    pub serial: String,
}
#[derive(Debug)]
pub struct Monitor {
    pub info: MonitorInfo,
    pub modes: Vec<Mode>,
    pub properties: PropMap,
}
impl From<MonitorRaw> for Monitor {
    fn from(val: MonitorRaw) -> Self {
        Monitor {
            info: MonitorInfo {
                connector: val.0 .0,
                vendor: val.0 .1,
                product_name: val.0 .2,
                serial: val.0 .3,
            },
            modes: val.1.into_iter().map(|val| val.into()).collect(),
            properties: val.2,
        }
    }
}
impl From<Monitor> for MonitorRaw {
    fn from(val: Monitor) -> Self {
        (
            (
                val.info.connector,
                val.info.vendor,
                val.info.product_name,
                val.info.serial,
            ),
            val.modes.into_iter().map(|mode| mode.into()).collect(),
            val.properties,
        )
    }
}

/// ID: the ID in the API
/// winsys_id: the low-level ID of this mode
/// width: the resolution
/// height: the resolution
/// frequency: refresh rate
/// flags: mode flags as defined in xf86drmMode.h and randr.h
pub(crate) type ModeRaw = (String, i32, i32, f64, f64, Vec<f64>, PropMap);
#[derive(Debug)]
pub struct Mode {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub refresh_rate: f64,
    pub preferred_scale: f64,
    pub supported_scales: Vec<f64>,
    pub properties: PropMap,
}
impl From<ModeRaw> for Mode {
    fn from(val: ModeRaw) -> Self {
        Mode {
            id: val.0,
            width: val.1,
            height: val.2,
            refresh_rate: val.3,
            preferred_scale: val.4,
            supported_scales: val.5,
            properties: val.6,
        }
    }
}
impl From<Mode> for ModeRaw {
    fn from(val: Mode) -> Self {
        (
            val.id,
            val.width,
            val.height,
            val.refresh_rate,
            val.preferred_scale,
            val.supported_scales,
            val.properties,
        )
    }
}

#[derive(Debug)]
pub struct MonitorUpdateInfo {
    pub connector: String,
    pub mode: String,
    pub properties: PropMap,
}
#[derive(Debug)]
pub struct LogicalMonitorUpdate {
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: Transform,
    pub is_primary: bool,
    pub monitors: Vec<MonitorUpdateInfo>,
}

pub(crate) type LogicalMonitorUpdateRaw = (i32, i32, f64, u32, bool, Vec<(String, String, PropMap)>);

impl From<LogicalMonitorUpdateRaw> for LogicalMonitorUpdate {
    fn from(val: LogicalMonitorUpdateRaw) -> Self {
        let monitors = val
            .5
            .into_iter()
            .map(|val| MonitorUpdateInfo {
                connector: val.0,
                mode: val.1,
                properties: val.2,
            })
            .collect();
        LogicalMonitorUpdate {
            x: val.0,
            y: val.1,
            scale: val.2,
            transform: Transform::try_from(val.3).unwrap(),
            is_primary: val.4,
            monitors,
        }
    }
}
impl From<LogicalMonitorUpdate> for LogicalMonitorUpdateRaw {
    fn from(val: LogicalMonitorUpdate) -> Self {
        let monitors: Vec<(String, String, PropMap)> = val
            .monitors
            .into_iter()
            .map(|info| (info.connector, info.mode, info.properties))
            .collect();
        (
            val.x,
            val.y,
            val.scale,
            val.transform as u32,
            val.is_primary,
            monitors,
        )
    }
}

impl LogicalMonitorUpdate {
    pub(crate) fn matches_existing_state(
        &self,
        comparison: &LogicalMonitor,
        hardware_monitors: &Vec<Monitor>,
    ) -> bool {
        let monitors = comparison
            .monitors
            .iter()
            .map(|monitor_info| MonitorInfo {
                connector: monitor_info.connector.to_string(),
                product_name: String::new(),
                vendor: String::new(),
                serial: String::new(),
            })
            .collect();
        LogicalMonitor {
            x: comparison.x,
            y: comparison.y,
            scale: comparison.scale,
            transform: comparison.transform,
            is_primary: comparison.is_primary,
            monitors,
            properties: PropMap::new(),
        };

        let expected_mode_matches = self.monitors.iter().all(|monitor| {
            if let Some(hardware_monitor) = hardware_monitors
                .iter()
                .find(|&m| m.info.connector == monitor.connector)
            {
                let default_properties = PropMap::new();
                let mode_properties = hardware_monitor
                    .modes
                    .iter()
                    .find(|mode| mode.id == monitor.mode)
                    .map(|mode| &mode.properties)
                    .unwrap_or_else(|| &default_properties);
                // Make sure current mode matches monitor mode

                let monitor_is_current: bool = prop_cast(mode_properties, "is-current").cloned().unwrap_or(false);
                let monitor_is_preferred: bool = prop_cast(mode_properties, "is-preferred").cloned().unwrap_or(false);
                monitor_is_current || monitor_is_preferred
            } else {
                false
            }
        });
        log::trace!(
            "Testing if state is already valid for \nmonitor: {:#?} against monitor\n{:#?}",
            self,
            comparison
        );

        let result = expected_mode_matches
            && (self.x == comparison.x)
            && (self.y == comparison.y)
            && (self.scale == comparison.scale)
            && (self.transform == comparison.transform)
            && (self.is_primary == comparison.is_primary)
            && (self.monitors.len() == comparison.monitors.len())
            && (self
                .monitors
                .iter()
                .zip(comparison.monitors.iter())
                .all(|(a, b)| a.connector == b.connector));

        log::trace!(
            "Mode matches: {}. Full matches: {}",
            expected_mode_matches,
            result
        );

        result
    }
}

/// Method enum representing possible methods to call states
#[derive(Debug, Clone, Copy)]
pub enum Method {
    Verify = 0,
    Temporary = 1,
    Persistent = 2,
}

/// Monitor transformations
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Transform {
    Normal = 0,
    Degrees90 = 1,
    Degrees180 = 2,
    Degrees270 = 3,
    Flipped = 4,
    Degrees90Flipped = 5,
    Degrees180Flipped = 6,
    Degrees270Flipped = 7,
}

impl TryFrom<u32> for Transform {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == (Transform::Normal as u32) => Ok(Transform::Normal),
            x if x == (Transform::Degrees90 as u32) => Ok(Transform::Degrees90),
            x if x == (Transform::Degrees180 as u32) => Ok(Transform::Degrees180),
            x if x == (Transform::Degrees270 as u32) => Ok(Transform::Degrees270),
            x if x == (Transform::Flipped as u32) => Ok(Transform::Flipped),
            x if x == (Transform::Degrees90Flipped as u32) => Ok(Transform::Degrees90Flipped),
            x if x == (Transform::Degrees180Flipped as u32) => Ok(Transform::Degrees180Flipped),
            x if x == (Transform::Degrees270Flipped as u32) => Ok(Transform::Degrees270Flipped),
            _ => Err(()),
        }
    }
}