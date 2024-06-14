/*
    Copyright 2024 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Resource limitations detection and config override parsing.

use config::builder::BuilderState;
use config::ConfigBuilder;
use serde::{Deserialize, Serialize};

use super::AppConfigDefaults;

/// Resource limitations override configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceLimitsConfig {
    /// Cores assigned to the app.
    cpus: f64,
    /// Memory assigned to the app in bytes.
    memory: Option<u64>,
}

impl AppConfigDefaults for ResourceLimitsConfig {
    /// Provide defaults for this part of the configuration.
    ///
    /// This will try to detect cgroup limitations of memory and CPU.
    fn set_defaults<T: BuilderState>(
        mut config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T> {
        let mut cpu_quota = None;
        let mut cpu_period = None;
        let mut memory_max = None;
        cgroups_rs::hierarchies::auto()
            .subsystems()
            .iter()
            .for_each(|subsystem| match subsystem.controller_name().as_str() {
                "cpu" => {
                    let cpu_controller: &cgroups_rs::cpu::CpuController = subsystem.into();
                    cpu_quota = cpu_controller
                        .cfs_quota()
                        .ok()
                        .and_then(|cfs_quota| u64::try_from(std::cmp::max(cfs_quota, 0)).ok());
                    cpu_period = cpu_controller.cfs_period().ok();
                }
                "memory" => {
                    let memory_controller: &cgroups_rs::memory::MemController = subsystem.into();
                    if let Ok(mem) = memory_controller.get_mem() {
                        if let Some(cgroups_rs::MaxValue::Value(mem_max_value)) = mem.max {
                            memory_max = u64::try_from(std::cmp::max(mem_max_value, 0)).ok();
                        }
                    }
                }
                _ => {
                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("Ignoring cgroup {}", subsystem.controller_name());
                    }
                }
            });
        let mut cpus = std::thread::available_parallelism().unwrap().get() as f64;
        if let Some(cpu_quota) = cpu_quota {
            if let Some(cpu_period) = cpu_period {
                cpus = cpu_quota as f64 / cpu_period as f64;
            }
        }
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Detected resource limits: cpus: {cpus}, memory: {memory_max:?}");
        }
        if let Some(memory) = memory_max {
            config_builder = config_builder
                .set_default(prefix.to_string() + "." + "memory", format!("{memory}"))
                .unwrap();
        }
        config_builder
            .set_default(prefix.to_string() + "." + "cpus", format!("{cpus}"))
            .unwrap()
    }
}

impl ResourceLimitsConfig {
    /** Supported level of parallelism.

       This roughly matches the number of full cores assigned to the app, but
       always returns at least 1.
    */
    pub fn available_parallelism(&self) -> usize {
        std::cmp::max(self.cpus as usize, 1)
    }

    /// CPU cores assigned to the app.
    pub fn cpus(&self) -> f64 {
        self.cpus
    }

    /// Memory assigned to the app in bytes.
    pub fn memory_bytes(&self) -> Option<u64> {
        self.memory
    }
}
