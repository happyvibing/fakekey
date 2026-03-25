# Release Checklist

## Pre-Release Checklist

### 1. Code Quality
- [ ] All tests pass: `cargo test`
- [ ] Code builds without warnings: `cargo build --release`
- [ ] Documentation is up to date
- [ ] Version number updated in `Cargo.toml`

### 2. Testing
- [ ] Test on multiple platforms (Linux, macOS, Windows)
- [ ] Test installation methods:
  - [ ] `cargo install fakekey`
  - [ ] Download from GitHub releases
  - [ ] Install script: `curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash`

### 3. Documentation
- [ ] README.md is updated
- [ ] README_CN.md is updated
- [ ] CHANGELOG.md (if exists) is updated
- [ ] Installation instructions are correct

### 4. Security
- [ ] No sensitive data in the repository
- [ ] Dependencies are up to date: `cargo update`
- [ ] Check for security advisories: `cargo audit`

## Release Process

### Automated Release (Recommended)
1. **Update version** in `Cargo.toml`
2. **Commit changes**: `git add . && git commit -m "Release v{version}"`
3. **Create tag**: `git tag v{version}`
4. **Push to GitHub**: `git push origin main --tags`
5. **GitHub Actions will automatically**:
   - Run tests
   - Build binaries for all platforms
   - Create GitHub release
   - Publish to crates.io

### Manual Release (if needed)
1. **Build locally** for all platforms
2. **Create GitHub release** manually
3. **Upload binaries** to release
4. **Publish to crates.io**: `cargo publish`

## Post-Release Checklist

### 1. Verification
- [ ] GitHub release is created with correct assets
- [ ] crates.io version is published
- [ ] Installation script works with new version
- [ ] Homebrew formula (if applicable) is updated

### 2. Announcements
- [ ] Update website/documentation
- [ ] Post on social media (optional)
- [ ] Notify users (if applicable)

### 3. Cleanup
- [ ] Remove temporary files
- [ ] Update development branch for next version
- [ ] Close related issues/PRs

## Version Format

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR.MINOR.PATCH** (e.g., 0.1.0, 0.1.1, 0.2.0)
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

## Quick Commands

```bash
# Test everything before release
cargo test
cargo build --release
cargo audit

# Create release (automated)
git add .
git commit -m "Release v0.1.2"
git tag v0.1.2
git push origin main --tags

# Check release status
gh release list
gh release view v0.1.2
cargo search fakekey
```

## Troubleshooting

### Common Issues

1. **GitHub Actions fails**
   - Check workflow logs: `gh run list`
   - Verify all tests pass locally
   - Check for syntax errors in workflow files

2. **crates.io publish fails**
   - Check if version already exists
   - Verify Cargo.toml format
   - Check network connectivity

3. **Install script issues**
   - Test platform detection
   - Verify download URLs are correct
   - Check checksum verification

4. **Binary build fails**
   - Check cross-compilation tools
   - Verify target platforms are supported
   - Check for platform-specific code issues
