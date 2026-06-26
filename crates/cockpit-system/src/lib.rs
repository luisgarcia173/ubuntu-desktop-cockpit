use cockpit_domain::SystemMetric;
use sysinfo::System;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemMetricsError {
    #[error("failed to collect system metrics")]
    Collect,
}

#[derive(Debug)]
pub struct SystemMetricsProvider {
    system: System,
}

impl Default for SystemMetricsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMetricsProvider {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn collect(&mut self) -> Result<SystemMetric, SystemMetricsError> {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let memory_total_bytes = self.system.total_memory();
        let memory_used_bytes = self.system.used_memory();
        let memory_usage_percent = if memory_total_bytes == 0 {
            0.0
        } else {
            (memory_used_bytes as f32 / memory_total_bytes as f32) * 100.0
        };

        Ok(SystemMetric {
            cpu_usage_percent: self.system.global_cpu_usage(),
            memory_total_bytes,
            memory_used_bytes,
            memory_usage_percent,
        })
    }
}
