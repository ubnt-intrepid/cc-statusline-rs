# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build           # debug build
cargo build --release # optimized build
cargo run             # run (pipe JSON on stdin)
cargo clippy          # lint
cargo fmt             # format
cargo test            # run tests
```

Test with sample input:

```bash
echo '{"model":{"display_name":"claude-sonnet-4-6"},"workspace":{"current_dir":"/home/user/project","git_branch":"main"},"rate_limits":{"five_hour":{"used_percentage":42.0},"seven_day":{"used_percentage":75.0}}}' | cargo run
```

## Additional Resources
- https://code.claude.com/docs/en/statusline
