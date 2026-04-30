use tokio_cron_scheduler::Job;
use crate::infrastructure::observability;
use sysinfo::System;
use std::sync::Mutex;
use tracing::info;

/// Builds a cron job that collects system and runtime metrics every 20 seconds.
pub fn build_metrics_job() -> Result<Job, anyhow::Error> {
    // Keep System object persistent to maintain CPU usage calculation state
    let sys = Mutex::new(System::new_all());
    let meter = observability::meter();
    
    // 1. Runtime & System Gauges
    let cpu_gauge = meter.f64_gauge("process.cpu.utilization")
        .with_description("Percentage of CPU used by the process")
        .build();
    let mem_gauge = meter.u64_gauge("process.memory.usage")
        .with_description("Total memory used by the process")
        .with_unit("By")
        .build();
    let tokio_workers = meter.u64_gauge("tokio.worker_threads.active")
        .with_description("Number of active Tokio worker threads")
        .build();
    let tokio_tasks = meter.u64_gauge("tokio.task.count")
        .with_description("Number of active Tokio tasks")
        .build();

    // Cron expression for every 20 seconds
    let job = Job::new_async("1/20 * * * * *", move |_uuid, _l| {
        let mut sys_lock = sys.lock().unwrap();
        
        // Refresh process-specific info
        let pid = sysinfo::get_current_pid().ok();
        let mut current_cpu = 0.0;
        let mut current_mem = 0;

        if let Some(id) = pid {
            sys_lock.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[id]), true);
            if let Some(p) = sys_lock.process(id) {
                current_cpu = p.cpu_usage() as f64;
                current_mem = p.memory();
            }
        }

        // 2. Get Tokio metrics via handle
        let handle = tokio::runtime::Handle::current();
        let t_metrics = handle.metrics();
        let workers = t_metrics.num_workers() as u64;
        let active_tasks = t_metrics.num_alive_tasks() as u64;

        // 3. Record metrics
        cpu_gauge.record(current_cpu, &[]);
        mem_gauge.record(current_mem, &[]);
        tokio_workers.record(workers, &[]);
        tokio_tasks.record(active_tasks, &[]);

        Box::pin(async move {
            info!(
                "Metrics collection: CPU: {:.2}%, RAM: {} bytes, Tokio Tasks: {}", 
                current_cpu, current_mem, active_tasks
            );
        })
    })?;

    Ok(job)
}
