use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::errors::ErpKernelError;

/// A Hybrid Logical Clock timestamp providing causality and total ordering
/// across distributed and offline-first nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HlcTimestamp {
    pub physical: i64,
    pub logical: u32,
    pub node_id: String,
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.physical
            .cmp(&other.physical)
            .then_with(|| self.logical.cmp(&other.logical))
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}-{:08x}-{}", self.physical, self.logical, self.node_id)
    }
}

impl FromStr for HlcTimestamp {
    type Err = ErpKernelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, '-').collect();
        if parts.len() != 3 {
            return Err(ErpKernelError::HlcParse(format!(
                "Invalid HLC timestamp format: '{}'",
                s
            )));
        }

        let physical = i64::from_str_radix(parts[0], 16)
            .map_err(|e| ErpKernelError::HlcParse(format!("Failed to parse physical time: {}", e)))?;
        let logical = u32::from_str_radix(parts[1], 16)
            .map_err(|e| ErpKernelError::HlcParse(format!("Failed to parse logical counter: {}", e)))?;
        let node_id = parts[2].to_string();

        Ok(HlcTimestamp {
            physical,
            logical,
            node_id,
        })
    }
}

/// Thread-safe Hybrid Logical Clock generator.
#[derive(Debug)]
pub struct HybridLogicalClock {
    node_id: String,
    state: Mutex<HlcState>,
}

#[derive(Debug, Clone)]
struct HlcState {
    physical: i64,
    logical: u32,
}

impl HybridLogicalClock {
    pub fn new(node_id: String) -> Self {
        let current_pt = Self::get_physical_time();
        Self {
            node_id,
            state: Mutex::new(HlcState {
                physical: current_pt,
                logical: 0,
            }),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    fn get_physical_time() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// Generate next timestamp for a local event.
    pub fn now(&self) -> HlcTimestamp {
        let pt = Self::get_physical_time();
        let mut state = self.state.lock();

        if pt > state.physical {
            state.physical = pt;
            state.logical = 0;
        } else {
            state.logical += 1;
        }

        HlcTimestamp {
            physical: state.physical,
            logical: state.logical,
            node_id: self.node_id.clone(),
        }
    }

    /// Update local clock upon receiving a remote HLC timestamp.
    pub fn update(&self, remote: &HlcTimestamp) -> HlcTimestamp {
        let pt = Self::get_physical_time();
        let mut state = self.state.lock();

        let prev_physical = state.physical;
        let prev_logical = state.logical;

        let max_physical = pt.max(prev_physical).max(remote.physical);

        let next_logical = if max_physical == prev_physical && max_physical == remote.physical {
            prev_logical.max(remote.logical) + 1
        } else if max_physical == prev_physical {
            prev_logical + 1
        } else if max_physical == remote.physical {
            remote.logical + 1
        } else {
            0
        };

        state.physical = max_physical;
        state.logical = next_logical;

        HlcTimestamp {
            physical: state.physical,
            logical: state.logical,
            node_id: self.node_id.clone(),
        }
    }
}
