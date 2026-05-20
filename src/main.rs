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

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else {
        format!("{}", n)
    }
}


fn main() {
    let input = io::read_to_string(io::stdin().lock()).unwrap_or_default();
    let status: StatusLine = serde_json::from_str(&input).unwrap_or_default();

    let sep = "|".dimmed();
    let mut parts: Vec<String> = Vec::new();

    // 1. Model name
    let model = status
        .model
        .as_ref()
        .and_then(|m| m.display_name.as_deref())
        .unwrap_or("Claude");
    parts.push(format!("{}", format!("[{}]", model).cyan()));

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
        Some(b) => parts.push(format!("{} ({})", basename.yellow(), b.green())),
        None => parts.push(format!("{}", basename.yellow())),
    }

    // 4. Context window usage
    let ctx_pct = status
        .context_window
        .as_ref()
        .and_then(|c| c.used_percentage)
        .unwrap_or(0.0);
    let ctx_pct_str = if ctx_pct >= 80.0 {
        format!("{:.0}%", ctx_pct).red().to_string()
    } else if ctx_pct >= 50.0 {
        format!("{:.0}%", ctx_pct).yellow().to_string()
    } else {
        format!("{:.0}%", ctx_pct).green().to_string()
    };
    parts.push(format!("ctx: {}", ctx_pct_str));

    // 5. Token usage
    let ctx = status.context_window.as_ref();
    let total_in = ctx.and_then(|c| c.total_input_tokens);
    let total_out = ctx.and_then(|c| c.total_output_tokens);
    let cache_read = ctx.and_then(|c| c.current_usage.as_ref()).and_then(|u| u.cache_read_input_tokens);
    let cache_write = ctx.and_then(|c| c.current_usage.as_ref()).and_then(|u| u.cache_creation_input_tokens);
    if total_in.is_some() || total_out.is_some() {
        let in_str = total_in.map(format_tokens).unwrap_or_else(|| "-".into());
        let out_str = total_out.map(format_tokens).unwrap_or_else(|| "-".into());
        parts.push(format!("tok: {}(in) {}(out)", in_str.cyan(), out_str.magenta()));
    }
    if cache_read.is_some() || cache_write.is_some() {
        let mut cache = String::from("cache:");
        if let Some(r) = cache_read {
            cache.push_str(&format!(" {}(r)", format_tokens(r).green()));
        }
        if let Some(w) = cache_write {
            cache.push_str(&format!(" {}(w)", format_tokens(w).yellow()));
        }
        parts.push(cache);
    }

    // 6. Rate limits (only available on Pro/Max plans)
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
        let mut items: Vec<String> = Vec::new();
        for (pct, label) in [(five_hour, "5h"), (seven_day, "7d")] {
            if let Some(p) = pct {
                let pct_str = format!("{:.0}%", p);
                let colored = if p >= 80.0 {
                    pct_str.red().to_string()
                } else if p >= 50.0 {
                    pct_str.yellow().to_string()
                } else {
                    pct_str.green().to_string()
                };
                items.push(format!("{}({})", colored, label));
            }
        }
        parts.push(format!("rate: {}", items.join(" ")));
    }

    println!("{}", parts.join(&format!(" {} ", sep)));
}
