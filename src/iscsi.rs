use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const CONFIGFS_ISCSI: &str = "/sys/kernel/config/target/iscsi";

#[derive(Debug, Clone)]
pub struct Lun {
    pub index: u32,
    /// e.g. `/dev/zvol/newtank/k8s/pvc-xxxx` (udev_path or fd_dev_name)
    pub device: String,
    /// Cumulative MB read by initiator (from scsi_tgt_port)
    pub read_mb: u64,
    /// Cumulative MB written by initiator
    pub write_mb: u64,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub target_iqn: String,
    pub initiator_iqn: String,
    pub luns: Vec<Lun>,
    /// Bytes/s read rate (computed by Collector)
    pub read_rate: f64,
    /// Bytes/s write rate (computed by Collector)
    pub write_rate: f64,
    /// Total cumulative read MB
    pub total_read_mb: u64,
    /// Total cumulative write MB
    pub total_write_mb: u64,
}

/// State held between collections for rate computation.
struct RateState {
    read_mb: u64,
    write_mb: u64,
    instant: Instant,
}

pub struct Collector {
    rate_cache: HashMap<(String, String), RateState>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            rate_cache: HashMap::new(),
        }
    }

    /// Traverse configfs and return current sessions with computed rates.
    pub fn collect(&mut self) -> Result<Vec<Session>> {
        let iscsi_path = Path::new(CONFIGFS_ISCSI);
        if !iscsi_path.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        for entry in fs::read_dir(iscsi_path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip non-IQN entries like cpus_allowed_list, discovery_auth, lio_version
            if !name.starts_with("iqn.")
                && !name.starts_with("eui.")
                && !name.starts_with("naa.")
            {
                continue;
            }

            let target_iqn = name.clone();
            let target_path = entry.path();

            // Each target has one or more tpgt_N directories
            for tpgt_entry in fs::read_dir(&target_path)
                .with_context(|| format!("reading {}", target_path.display()))?
            {
                let tpgt_entry = tpgt_entry?;
                let tpgt_name = tpgt_entry.file_name().to_string_lossy().to_string();
                if !tpgt_name.starts_with("tpgt_") {
                    continue;
                }

                let tpgt_path = tpgt_entry.path();
                let dynamic_sessions_path = tpgt_path.join("dynamic_sessions");

                if !dynamic_sessions_path.exists() {
                    continue;
                }

                let initiators = read_dynamic_sessions(&dynamic_sessions_path)?;
                if initiators.is_empty() {
                    continue;
                }

                let luns = collect_luns(&tpgt_path)?;
                let total_read_mb: u64 = luns.iter().map(|l| l.read_mb).sum();
                let total_write_mb: u64 = luns.iter().map(|l| l.write_mb).sum();

                // For now one session per tpgt (single initiator shown)
                let initiator_iqn = initiators.join(", ");
                let key = (target_iqn.clone(), initiator_iqn.clone());

                let (read_rate, write_rate) = self.compute_rates(&key, total_read_mb, total_write_mb);

                sessions.push(Session {
                    target_iqn: target_iqn.clone(),
                    initiator_iqn,
                    luns,
                    read_rate,
                    write_rate,
                    total_read_mb,
                    total_write_mb,
                });
            }
        }

        sessions.sort_by(|a, b| a.target_iqn.cmp(&b.target_iqn));
        Ok(sessions)
    }

    fn compute_rates(
        &mut self,
        key: &(String, String),
        read_mb: u64,
        write_mb: u64,
    ) -> (f64, f64) {
        let now = Instant::now();
        if let Some(prev) = self.rate_cache.get(key) {
            let elapsed = now.duration_since(prev.instant).as_secs_f64();
            let read_rate = if elapsed > 0.0 {
                (read_mb.saturating_sub(prev.read_mb) as f64 * 1024.0 * 1024.0) / elapsed
            } else {
                0.0
            };
            let write_rate = if elapsed > 0.0 {
                (write_mb.saturating_sub(prev.write_mb) as f64 * 1024.0 * 1024.0) / elapsed
            } else {
                0.0
            };
            self.rate_cache.insert(
                key.clone(),
                RateState { read_mb, write_mb, instant: now },
            );
            (read_rate, write_rate)
        } else {
            self.rate_cache.insert(
                key.clone(),
                RateState { read_mb, write_mb, instant: now },
            );
            (0.0, 0.0)
        }
    }
}

/// Read `dynamic_sessions` file, filtering null bytes, return non-empty IQNs.
fn read_dynamic_sessions(path: &PathBuf) -> Result<Vec<String>> {
    let raw = fs::read(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let filtered: Vec<u8> = raw.into_iter().filter(|&b| b != 0).collect();
    let content = String::from_utf8_lossy(&filtered);
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

/// Collect LUN info from `tpgt_path/lun/lun_N/`.
fn collect_luns(tpgt_path: &Path) -> Result<Vec<Lun>> {
    let lun_dir = tpgt_path.join("lun");
    if !lun_dir.exists() {
        return Ok(Vec::new());
    }

    let mut luns = Vec::new();

    for entry in fs::read_dir(&lun_dir)? {
        let entry = entry?;
        let entry_name = entry.file_name().to_string_lossy().to_string();
        if !entry_name.starts_with("lun_") {
            continue;
        }

        let idx: u32 = entry_name
            .strip_prefix("lun_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let lun_path = entry.path();

        let device = resolve_lun_device(&lun_path).unwrap_or_else(|_| String::from("unknown"));

        let stats_base = lun_path.join("statistics/scsi_tgt_port");
        let read_mb = read_u64_file(&stats_base.join("read_mbytes")).unwrap_or(0);
        let write_mb = read_u64_file(&stats_base.join("write_mbytes")).unwrap_or(0);

        luns.push(Lun { index: idx, device, read_mb, write_mb });
    }

    luns.sort_by_key(|l| l.index);
    Ok(luns)
}

/// Follow the first symlink inside `lun_path` to find the backing store directory,
/// then read `udev_path` (or `fd_dev_name` for fileio).
fn resolve_lun_device(lun_path: &Path) -> Result<String> {
    for entry in fs::read_dir(lun_path)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            let target = fs::canonicalize(entry.path())?;
            // Try udev_path first (iblock), then fd_dev_name (fileio)
            for name in &["udev_path", "fd_dev_name"] {
                let p = target.join(name);
                if p.exists() {
                    let val = fs::read_to_string(&p)?;
                    let val = val.trim().to_string();
                    if !val.is_empty() {
                        return Ok(val);
                    }
                }
            }
            // Fall back to the target directory name
            return Ok(target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| String::from("unknown")));
        }
    }
    Ok(String::from("unknown"))
}

fn read_u64_file(path: &Path) -> Result<u64> {
    let s = fs::read_to_string(path)?;
    Ok(s.trim().parse()?)
}
