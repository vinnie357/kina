# Contributing to kina

Thank you for your interest in contributing to kina!

## Getting Started

1. Fork the repository at https://github.com/vinnie357/kina
2. Clone your fork and set up the development environment:
   ```bash
   git clone https://github.com/<your-username>/kina.git
   cd kina
   mise install
   mise run setup
   ```
3. Find an open task: `bees ready`
4. Create a branch: `git checkout -b type/description`

## Development Workflow

- Run `mise run check` to verify the build
- Run `mise run test` to run unit tests
- Run `mise run lint` to run clippy
- Run `mise run fmt` to format code

See [CLAUDE.md](CLAUDE.md) and [AGENTS.md](AGENTS.md) for the full workflow, commit conventions, and task tracking instructions.

## Submitting Changes

1. Ensure tests pass: `mise run ci`
2. Commit using the format: `type(scope): description`
3. Open a pull request against `main`

## License

By contributing, you agree that your contributions will be licensed under the same [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) dual license as the project.
