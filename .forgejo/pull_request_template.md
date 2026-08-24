<!-- Please put the pull request description above. -->

---

## Checklist

The [CONTRIBUTING.md](https://github.com/monyacode/monyacode/src/branch/main/CONTRIBUTING.md) contains
information that will be helpful to first time contributors.

### Compliance

- [ ] I have read the
      [code of conduct](https://github.com/monyacode/monyacode/src/branch/main/CODE_OF_CONDUCT.md) and
      agree with them.

### Tests for Rust changes

<!-- Can be removed for non-Rust changes. -->

- I ran...
  - [ ] `cargo run --profile dev` to check for issues with app building.
  - [ ] `cargo fmt --all` and `./script/clippy` to check formatting and linting.

### Release notes

- [ ] This change will be noticed by a MonyaCode user (feature, bug fix, performance, etc.). I suggest to
      include a release note for this change.
- [ ] This change is not visible to a MonyaCode user (refactor, dependency upgrade, etc.). I think there
      is no need to add a release note for this change.
