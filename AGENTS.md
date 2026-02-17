# Repository Guidelines

## Project Structure & Module Organization

- `src/` contains application code; `src/main.rs` is the binary entry point and `src/lib.rs` holds shared logic.
- Keep UI code in `src/ui/` (for example, `main_window.rs`, `virtual_list.rs`, `settings_window.rs`).
- Feature modules live in `src/clipboard/`, `src/db/`, `src/settings/`, `src/tray/`, and `src/shortcuts/`.
- Place static resources in `assets/`.
- `target/` is generated build output and must not be committed.

## Build, Test, and Development Commands

- `cargo run`: build and launch the app locally.
- `cargo build --release`: produce an optimized production binary.
- `cargo test`: run unit tests.
- `cargo fmt`: format code using `rustfmt` before opening a PR.

## Coding Style & Naming Conventions

- Follow Rust 2024 idioms and keep formatting consistent with `rustfmt`.
- Naming conventions: `snake_case` for modules/functions, `CamelCase` for structs/enums/traits, `SCREAMING_SNAKE_CASE` for constants.
- Keep boundaries clean: UI behavior belongs in `src/ui/`; persistence/query logic belongs in `src/db/`.

## Testing Guidelines

- No dedicated integration test suite exists yet; prefer module-local unit tests with `#[cfg(test)]`.
- Use descriptive test names such as `fn saves_clipboard_entry()`.
- Validate changes with `cargo test` and include reproduction steps in the PR for behavior that is hard to unit test (for example, tray or shortcut behavior).

## Commit & Pull Request Guidelines

- With minimal history, use clear conventional-style messages: `feat:`, `fix:`, `chore:`, `refactor:`.
- Keep commits focused and atomic; avoid bundling unrelated changes.
- PRs should include:
  - a brief summary of what changed and why,
  - explicit verification steps (commands run),
  - screenshots or GIFs for visible UI updates.

## Configuration & Platform Notes

- Runtime settings path: `~/Library/Application Support/clipboard-manager/settings.json`.
- The app currently targets macOS. Keep OS-specific logic explicit and isolated when adding integrations (tray, shortcuts, clipboard formats).
