# Contributing

Thanks for helping improve this project.

## Keep PRs focused

Each PR should represent one logical change.

- Keep unrelated work in separate PRs (for example: gameplay fixes, refactoring, and UI restyling).
- If a change is large, the scope is uncertain, or anything else is unclear, open an issue first for discussion.

## What to include in a PR

Please include:

- What changed.
- Why it changed.
- What you tested.

If behavior or visuals changed, add screenshots or a short video.

If the change is a significant visual or aesthetic shift, propose it in an issue before implementation.

## Coding agents

Coding agents are welcome. Many existing features in this project were developed with their help.

- Project-specific coding-agent guidelines such as `AGENTS.md` are not currently tracked here. Contributors can use their own setup to keep more freedom in their workflow.
- If you think a coding-agent guideline is project-wide rather than personal preference, open an issue or submit a PR so it can be discussed.
- You are still responsible for manually verifying gameplay behavior before opening a PR.

## Performance changes

If you claim a performance improvement, include measurable before/after results from your own platform.

Include all of the following:

- Platform and version (OS, game/build version).
- Hardware (CPU, GPU, RAM).
- Test setup (scene, resolution, graphics settings, and method).
- Before/after numbers (FPS and/or frame time).
- Tradeoffs or regressions you noticed.

Performance claims without numbers are treated as unverified.

## Before opening a PR

Confirm the following:

- The build succeeds.
- The game launches.
- The Vulkan validation layer does not report complaints during runtime.
- The changed behavior was tested directly.

## Review expectations

Maintainers may request changes, ask for a PR to be split, or close a PR that does not fit the project's direction.

Please keep all discussion respectful and constructive.
