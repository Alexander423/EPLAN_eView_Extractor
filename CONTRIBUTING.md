# Contributing to EPLAN eVIEW Extractor

Thank you for your interest in contributing! This project aims to make PLC variable extraction from EPLAN eVIEW as painless as possible.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/EPLAN_eView_Extractor.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Test thoroughly
6. Submit a pull request

## Development Setup

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- Google Chrome
- Git

### Building
```bash
cargo build
cargo test
cargo run
```

### Code Style
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Follow existing patterns in the codebase
- Write clear commit messages

## Types of Contributions

### Bug Reports
When reporting bugs, please include:
- Your operating system and version
- Steps to reproduce the issue
- Expected vs actual behavior
- Screenshots if applicable
- Any error messages or logs

### Feature Requests
- Check if the feature already exists or is planned
- Explain the use case and why it would be valuable
- Consider implementation complexity
- Be open to discussion about the approach

### Code Contributions
- Start with smaller changes to get familiar with the codebase
- Focus on one feature/fix per pull request
- Include tests for new functionality
- Update documentation if needed
- Make sure the build passes

## Project Structure

```
src/
├── main.rs           # Application entry point
├── ui/               # GUI components (egui)
├── scraper/          # Web automation (thirtyfour)
├── auth/             # Microsoft authentication
├── models/           # Data structures
├── export/           # File export functionality
└── config.rs         # Configuration management
```

## Coding Guidelines

- **Performance**: This tool should be fast - avoid unnecessary allocations
- **Reliability**: Error handling is critical - always use proper error types
- **User Experience**: The GUI should be intuitive and responsive
- **Documentation**: Comment complex logic and public APIs

## Testing

- Test on different Windows versions if possible
- Verify both light and dark themes work
- Test with different EPLAN projects
- Check that exports work correctly
- Ensure the tool gracefully handles network issues

## Pull Request Process

1. Update the README if you've added features
2. Ensure your code follows the existing style
3. Add tests for new functionality
4. Make sure all tests pass
5. Update version numbers if needed
6. Write a clear description of your changes

## Community

- Be respectful and professional
- Help others when possible
- Report security issues privately
- Focus on constructive feedback

## Questions?

- Open an issue for questions about the codebase
- Check existing issues before creating new ones
- Be patient - this is maintained by volunteers

## License

By contributing, you agree that your contributions will be licensed under the MIT License.