//! Restart — kill the running hmanlab gateway/daemon and start a fresh one.

use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};

const STOP_TIMEOUT_SECS: u64 = 10;

fn find_running_pids() -> Result<Vec<u32>> {
    let my_pid = std::process::id();
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("hmanlab")
        .output()
        .context("failed to run pgrep")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter(|pid| *pid != my_pid)
        .collect();

    Ok(pids)
}

fn stop_processes(pids: &[u32]) -> Result<()> {
    if pids.is_empty() {
        bail!("hmanlab is not running");
    }

    for pid in pids {
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output();
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(STOP_TIMEOUT_SECS);
    let mut remaining = pids.to_vec();

    while std::time::Instant::now() < deadline && !remaining.is_empty() {
        remaining.retain(|pid| {
            let check = Command::new("kill").arg("-0").arg(pid.to_string()).output();
            check.map(|o| o.status.success()).unwrap_or(false)
        });

        if !remaining.is_empty() {
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    for pid in &remaining {
        let _ = Command::new("kill")
            .arg("-KILL")
            .arg(pid.to_string())
            .output();
    }

    if !remaining.is_empty() {
        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn start_gateway() -> Result<u32> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;

    let child = Command::new("nohup")
        .arg(&exe)
        .arg("gateway")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to start hmanlab gateway")?;

    Ok(child.id())
}

pub(crate) fn cmd_restart() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("Restarting hmanlab v{version}...");

    let pids = find_running_pids()?;

    if pids.is_empty() {
        println!("No running hmanlab process found. Starting fresh...");
    } else {
        println!(
            "Stopping {} process(es): {}",
            pids.len(),
            pids.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        stop_processes(&pids)?;
        println!("Stopped.");
    }

    let new_pid = start_gateway()?;
    println!("Started hmanlab gateway (PID {})", new_pid);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_running_pids_runs() {
        let result = find_running_pids();
        assert!(result.is_ok(), "pgrep should not panic");
    }

    #[test]
    fn test_stop_empty_pids_errors() {
        let result = stop_processes(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn test_find_excludes_self() {
        let pids = find_running_pids().unwrap();
        let my_pid = std::process::id();
        assert!(!pids.contains(&my_pid));
    }
}
