//! Persistent usage tracking — append-only JSONL log with time-period aggregation.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub ts: DateTime<Utc>,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
    pub request_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AllStats {
    pub today: UsageStats,
    pub week: UsageStats,
    pub month: UsageStats,
    pub total: UsageStats,
}

pub struct UsageLogger {
    file_path: PathBuf,
}

impl UsageLogger {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file_path: data_dir.join("usage.jsonl"),
        }
    }

    pub fn log(&self, entry: &UsageEntry) -> std::io::Result<()> {
        let dir = self.file_path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        let line = serde_json::to_string(entry).unwrap_or_default();
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn aggregate(&self) -> AllStats {
        let entries = self.load_entries();
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let week_start = (now - chrono::Duration::days(7)).date_naive().and_hms_opt(0, 0, 0).unwrap();
        let month_start = now.with_day(1).unwrap_or(now).date_naive().and_hms_opt(0, 0, 0).unwrap();

        let mut stats = AllStats::default();

        for e in &entries {
            let entry_date = e.ts.naive_utc();

            // Total
            stats.total.prompt_tokens += e.prompt_tokens;
            stats.total.completion_tokens += e.completion_tokens;
            stats.total.cost_usd += e.cost_usd;
            stats.total.request_count += 1;

            // Month
            if entry_date >= month_start {
                stats.month.prompt_tokens += e.prompt_tokens;
                stats.month.completion_tokens += e.completion_tokens;
                stats.month.cost_usd += e.cost_usd;
                stats.month.request_count += 1;
            }

            // Week
            if entry_date >= week_start {
                stats.week.prompt_tokens += e.prompt_tokens;
                stats.week.completion_tokens += e.completion_tokens;
                stats.week.cost_usd += e.cost_usd;
                stats.week.request_count += 1;
            }

            // Today
            if entry_date >= today_start {
                stats.today.prompt_tokens += e.prompt_tokens;
                stats.today.completion_tokens += e.completion_tokens;
                stats.today.cost_usd += e.cost_usd;
                stats.today.request_count += 1;
            }
        }

        stats
    }

    fn load_entries(&self) -> Vec<UsageEntry> {
        if !self.file_path.exists() {
            return Vec::new();
        }
        let data = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        data.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }
}
