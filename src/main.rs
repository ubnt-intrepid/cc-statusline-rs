#![allow(dead_code)]

use std::io;
use std::process::Command;

use owo_colors::OwoColorize;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
struct StatusLine {
    model: Option<Model>,
    workspace: Option<Workspace>,
    context_window: Option<ContextWindow>,
    exceeds_200k_tokens: Option<bool>,
    cost: Option<Cost>,
    vim: Option<Vim>,
    session_id: Option<String>,
    session_name: Option<String>,
    transcript_path: Option<String>,
    version: Option<String>,
    output_style: Option<OutputStyle>,
    agent: Option<Agent>,
    rate_limits: Option<RateLimits>,
    worktree: Option<Worktree>,
}

#[derive(Debug, Deserialize)]
struct Model {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    current_dir: Option<String>,
    project_dir: Option<String>,
    added_dirs: Option<Vec<String>>,
    git_worktree: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextWindow {
    total_input_tokens: Option<u64>,
    total_output_tokens: Option<u64>,
    context_window_size: Option<u64>,
    used_percentage: Option<f64>,
    remaining_percentage: Option<f64>,
    current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Deserialize)]
struct CurrentUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Cost {
    total_cost_usd: Option<f64>,
    total_duration_ms: Option<u64>,
    total_api_duration_ms: Option<u64>,
    total_lines_added: Option<i64>,
    total_lines_removed: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Vim {
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OutputStyle {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Agent {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RateLimits {
    five_hour: Option<RateLimit>,
    seven_day: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
struct RateLimit {
    used_percentage: Option<f64>,
    resets_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Worktree {
    name: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    original_cwd: Option<String>,
    original_branch: Option<String>,
}

fn progress_bar(pct: f64) -> String {
    const WIDTH: usize = 10;
    let filled = ((pct / 100.0) * WIDTH as f64).round() as usize;
    let filled = filled.min(WIDTH);
    let empty = WIDTH - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    if pct >= 80.0 {
        bar.red().to_string()
    } else if pct >= 50.0 {
        bar.yellow().to_string()
    } else {
        bar.green().to_string()
    }
}

fn main() {
    let input = io::read_to_string(io::stdin().lock()).unwrap_or_default();
    let status: StatusLine = serde_json::from_str(&input).unwrap_or_default();

    let sep = "|".dimmed();
    let mut line1: Vec<String> = Vec::new();
    let mut line2: Vec<String> = Vec::new();

    // 1. Model name
    let model = status
        .model
        .as_ref()
        .and_then(|m| m.display_name.as_deref())
        .unwrap_or("Claude");
    line1.push(format!("{}", format!("[{}]", model).cyan()));

    // 2. Basename of the project directory
    let dir = status
        .workspace
        .as_ref()
        .and_then(|w| w.project_dir.as_deref())
        .unwrap_or("?");
    let basename = dir.rsplit('/').next().unwrap_or(dir);

    // 3. Git branch (workspace.git_branch は存在しないため git コマンドで取得)
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match branch.as_deref() {
        Some(b) => line1.push(format!("{} ({})", basename.yellow(), b.green())),
        None => line1.push(format!("{}", basename.yellow())),
    }

    // 4. Context window usage
    let ctx_pct = status
        .context_window
        .as_ref()
        .and_then(|c| c.used_percentage)
        .unwrap_or(0.0);
    line2.push(format!("ctx:{} {:.0}%", progress_bar(ctx_pct), ctx_pct));

    // 5. Rate limits (only available on Pro/Max plans)
    let five_hour = status
        .rate_limits
        .as_ref()
        .and_then(|r| r.five_hour.as_ref())
        .and_then(|l| l.used_percentage);
    let seven_day = status
        .rate_limits
        .as_ref()
        .and_then(|r| r.seven_day.as_ref())
        .and_then(|l| l.used_percentage);

    if five_hour.is_some() || seven_day.is_some() {
        let mut rate = String::from("rate ");
        if let Some(pct) = five_hour {
            rate.push_str(&format!("5h:{} {:.0}%", progress_bar(pct), pct));
        }
        if let Some(pct) = seven_day {
            if five_hour.is_some() {
                rate.push(' ');
            }
            rate.push_str(&format!("7d:{} {:.0}%", progress_bar(pct), pct));
        }
        line2.push(rate);
    }

    println!("{}", line1.join(&format!(" {} ", sep)));
    if !line2.is_empty() {
        println!("{}", line2.join(&format!(" {} ", sep)));
    }
}
