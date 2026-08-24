<!-- prettier-ignore-start -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Added

- Add `Toggle` actions to all the side panels (#389) by @ycna07
- Show added and deleted line counts per entry and in total in the git panel,
  behind the `git_panel.diff_stats` setting (zed#49519) (zed#58018)
- Add a branch picker filter for all, local, or remote branches (zed#60857)
- Add `git::ToggleFillCommitEditor` to expand the commit editor to fill the
  git panel, bound to `alt-shift-escape` (zed#55043) (zed#55565) (zed#60368)
  (zed#59901)
- Add git history views, both for folders and the current state
- Add action `git:show history`

## Removed

- Removed (partial) HTML support from the Markdown preview

## Changed

- Remove avatar fetching from git crate (#388)
- Use cosmic-text instead of font-kit for font selection
- Remove use of private MacOS APIs and drop support for MacOS < 12 (zed#56315)
- Tweaked ferra theme colors (#391) (#399) by @BillyDM
- Make clicking the last commit in the git panel open the history view
- Open commits in an adjacent split and preview tab from the history view

## Fixed

- Fix FsWatcher::add to correctly check watched ancestors on macOS (#406) by @sir_nacnud
- Don't remove window handle from workspace store if another workspace refers to it (#407) by @jasonmader
- Remove mention of LLMs in docs/remote-development (#396) by @mayas
- Downgrade embed-resource to 3.0.9 for Windows compatibility (#390)
- Fix multiple menus entries on some Linux distributions (#394) by @koru
- Fix Gentoo packaging parse errors (#397) by @koru
- Use cfg_select where reasonable (#387) by @ltdk
- Fix stale git status entries left behind when a directory is moved or removed
  (zed#59934)
- Reload git state when a watcher rescan covers a repository (zed#61541)
- Render file names in the git panel on a single line (zed#60862)
- Avoid dropping git repositories during watcher rescans (zed#59976)
- Refresh git state after committing, so the panel no longer lists the
  committed files until the next reload
- Enable the commit button when the editor is empty and a commit message is
  suggested
- Refresh git state after resetting, fetching and pulling, so the panel and
  the ahead/behind counts stay current (zed#60590)
- Use `buffer_font_size` for the commit message editor instead of the smaller
  UI font, and lay the commit container out so it holds up at larger font
  sizes (zed#55233) (zed#60331)
- Size the commit modal from the buffer font it renders in, rather than the
  narrower UI font, so it fits the commit message
- Canonicalize the placement of ambiguous diff hunks, fixing hunks that
  rendered as staged while git had them unstaged (zed#60584)
- Stop Discard Tracked Changes from also discarding unstaged files
  (zed#61608)
- Use a specific label and prompt when reverting a deleted file, rather than
  saying "restore" for modified files too (zed#61554)
- Stop git panel keybindings from intercepting typing in the branch and
  repository selectors (zed#61282)
- Align the commit button back to the right of the commit editor footer
- Pass `.` rather than an empty pathspec when collecting diff stats for the
  repository root, which git rejects
- Don't create text system or wgpu context if headless (#361)
- Improved smooth scrolling support
- Fix incorrect jsonc suffix on jupyter kernels (#365)
- Fix X11 keyboard state synchronization (zed#53903) (#440)

## [3.2.0] - 2026-07-27

## Changed

- Also look for gram-remote-server in $PATH (#384) by @icewind

## Fixed

- Fix Windows build

## [3.1.0] - 2026-07-26

## Added

- Add [ferra](https://github.com/casperstorm/ferra) and
  [zedokai](https://github.com/slymax/zedokai) themes (#371) by @koru
- Add notes about `ethical-foss/open-slopware` (#376) by @koru
- Add option to hide the title bar (#375) (zed#37428) by @ycna07

## Changed

- Preserve process PATH during shell environment capture (#366) by @dnlzro
- Complete the objc2 rewrite of the Mac gpui platform layer
- Update to rust 1.97
- Upgrade to wgpu 30.0.0

## Fixed

- Disable indirect call validation on Mac OS (#360)
- Update root_path for compatibility with language servers (zed#48587) (#362)
- Register toggle inline values action (zed#58921) (#373)
- Fix emoji rendering on macOS (#378) from @Petrosz007
- Fix npm v12 output deserialization (#381) (zed#60798)
- Fix broken language server settings UI for Node.js (#379)

## [3.0.1] - 2026-06-30

## Added

- Set `GRAM_UPDATE_EXPLANATION` variable for external distributions (#357) by @koru

## Fixed

- Fix missing CGDirectDisplayID method on Mac OS 15 (#359)
- docs: Fix broken link in docs
- docs: Fix vim indent keymap tip

## [3.0.0] - 2026-06-29

## Changed

- Port Mac from custom Metal renderer to using wgpu
- Use cosmic-text for text layout on both Linux and Mac OS
- Upgrade cosmic-text to 0.19
- Switch default system font to DejaVu Sans on Linux
- Backport cosmic-text layout fixes from Zed
- Always display "Return to Onboarding" on Welcome screen
- Gate Prettier download behind allow option in LSP configuration
- Gate NPM package installation behind allow option in LSP configuration

## Added

- Add `editor.line_number_scale` setting for smaller line numbers
- Add support for variable weight fonts to cosmic text system
- Add bold & italic variants to default monospace font (Myna)

## Fixed

- Prevent table cutoff in markdown preview (#332) by @kmkkiii
- Restore launchpad when using CLI without args (#340) by @ycna07
- Build system / CI improvements (#335) by @koru
- Add installation instructions for MSYS2 (#337) by @ognevny
- Fix potential hang when using clipboard on Linux / Wayland (zed#58826) by @dantai
- Handle macOS file open events (#353) by @pixelkarma
- Normalize line endings (#354) by @gluca
- Update settings immediately without relying on FS watcher notification (#334)
- Watch files that don't exist yet by polling (like settings.jsonc) (#334)
- Trigger full rescan when focusing project panel
- Refresh git panel when focused
- Refresh entries for paths whenever active item changes
- Better version handling for debug builds
- Drop git2-rs dependency (zed#53453) (#331) (#234)
- Actually toggle terminal on Toggle action
- Don't generate minidumps unless GRAM_GENERATE_MINIDUMPS is set
- Update to rust 1.96
- Bump UI label line height to 1.125 (+2/16px) (#321)
- Don't enable relative line numbers in multibuffers
- Fix stacked borrows violation in scheduler (zed#56850)
- Use buffer font size for outlines (since they use buffer font)
- Fix check for nixd LSPS (#347)
- Prefer to open in focused window (#350)
- Fix LSP provided by extensions not working in remote sessions (zed#56487)
- Fix loading tasks from extensions
- Make the Minimal keymap a bit less minimal to reduce confusion (#346)
- Fix cmd-e on macOS to behave more like it should (zed#54451)
- Save buffers before debug run (#338)
- Use objc2 for window prompts (zed#59572)
- Fix recent files picker out of sync when containing paths that no longer exist
- Fix traffic light position after appearance changes (Mac) (zed#59712)
- Roto has moved to // for comments in 0.11

## [2.2.0] - 2026-06-08

### Added

- Add a basic picker for recent files (`recent_files::Toggle`)
- Sync kill ring with system clipboard (#216)
- Add option to control kill ring sync (#216)
- Add separate KillLine and KillRingCut (kill-region) actions (#216)
- [Roto][roto] language support
- Add 'View File History' to tab context menu
- Add option to change status bar icon size
- Add option to save edited buffer before a task (#326) by @coma94
- Add actions to move to start and end of larger syntax node (zed#45331)
- Update git2 to 0.21.0 to support SHA-256 object repos (#325) (zed#57587)
- Add action to activate tab by index (`workspace::ActivateTab`)

### Fixed

- Fix nav history when clicking line numbers to jump in multibuffer
- Fix missing app menu button (the sword)
- Fix vim change surrounds for MiniQuotes, MiniBrackets, and AnyQuotes (zed#51067)
- Prefer Mailbox present mode on Wayland to avoid FIFO stalls (#318) (zed#57077)
- Fall back to poll watcher if watch creation fails
- Fix panic in git_panel render_status_entry (#323) by @SED4906
- Use eslint language server if installed as vscode-eslint-language-server
- Fix LSP menu anchor when status bar is at top
- Reduce git CPU usage on large repos (zed#56406)
- Downgrade notify to 9.0.0-rc.3 to fix broken FSEvent watches (Mac OS)

### Changed

- Make clicking SHA in git graph commit pane open commit diff view
- Try to improve UI font sizing and spacing (#321)
- Make diagnostics indicator toggle diagnostics editor

[roto]: https://roto.docs.nlnetlabs.nl/en/stable/

## [2.1.2] - 2026-05-23

- AltGr: Don't modify Modifiers to avoid breaking Linux (#307)

## [2.1.1] - 2026-05-21

### Fixed

- Fix git status message aligned incorrectly (#306)
- Fix .tar.gz unpack for extensions (#295)
- Debounce search by 100ms (#106)
- Workaround for flickering in wayland/wlroots (#310) by @nicoco
- Ignore eslint/status notification
- Only use option_as_alt setting on Mac OS (#307)

## [2.1.0] - 2026-05-18

### Added

- Add Supertab fallback mode setting (tab or indent) (#286)
- Add Sumi-Light theme via [Calvin Jackson, Nishant Dahiya][sumi-light]
- Add everforest light theme (#301) by @nirogu
- Add setting to treat right option as alt on Mac OS (#206)
- Improve outlines for Go (#299) by @danvolchek
- CI test and build for Windows, thanks to Zoey Rust

### Fixed

- Skip link to openssh.org when finding pull request link in git push output (#287)
- Fix AltGr / Right option detection on Mac OS (#206)
- Use window_decorations setting for settings UI and about (#297)
- Fix incorrect column display when indenting with tabs (#289)
- Support /path/to/file.txt(ln, col) style paths in terminal (second attempt) (#75)
- Fix eslint LSP not checking if server is downloaded (#296)

### Changed

- tasks: show command before output, also show args (#304) by @danvolchek

[sumi-light]: https://github.com/LogicSatinn/sumi-light-zed

## [2.0.0] - 2026-05-11

### Breaking

- Website now at <https://gram-editor.com>
- Downloaded language servers no longer auto update (#267)
- Emmyluadoc tree-sitter grammar was removed (it was broken)
- Modified default settings to be more sensible (#184)
- Removed download support for superhtml on Linux (#194) (it was broken)
- Removed settings for features that had been removed

### Added

- Add "Tight" line spacing for project panel
- Add Arch dependency check (#189) by @theDoctor
- Add Arch package build (#177) by @theDoctor
- Add optional FS poll watcher (based on [poll])
- Add tree-sitter grammar for git rebase
- RPM repository now available
- Add Refresh Folders command (#207) (zed#46291)
- Add SelectInsideDelimiters and SelectAroundDelimiters actions (#217) by @subeax
- Use patched protobuf-src for Windows support (#202) by @voltagex
- Added a menu item in the buffer and tab right-click menu for
  opening Markdown and SVG files in the preview tab. (zed#47821)
- Add ability to rename terminal tabs (zed#45800)
- Add Mermaid diagram support to Markdown preview (zed#49064)
- Add ScrollToTop and ScrollToBottom actions in Markdown preview (zed#50460)
- Add Kintsugi theme via [Giacomo Cavalieri][kintsugi]
- Add support for natively installed ESLint and Typescript LSPs (#229) by @brib
- Autoclose quotes in SQL (#183)
- Add ctrl-x g to toggle Git panel in Emacs keymap (#154)
- Update Arch Linux in readme; Add Alpine Linux (#249) by @msrd0
- Open file by absolute path in file finder (#212)
- Add an option to hide the unsaved indicator (#266) by @bux
- Make LSP auto-updating an opt-in setting (#267) by @bux
- Add option to move status bar to top of window
- Improve support for VSCodium task/debug import (#261)
- Smooth scrolling animation support (#140) (zed#44827)
- Show active file name in the status bar (zed#52381)
- List all buffers in tab switcher (zed#47079)
- Add ctrl/cmd-shift-o key binding to open tab switcher
- Add pane join actions for vim-like split close workflows (zed#50035)
- Make filenames clickable in git graph

[poll]: https://github.com/lilith/zed/commit/0334469a57a20586b28b86187028acd36559a9d3
[kintsugi]: https://github.com/giacomocavalieri/kintsugi-zed

### Removed

- Remove unimplemented CLI dev_server_token argument
- Remove emmyluadoc tree-sitter grammar; it can be pulled in via extension instead
- Remove avatar from git graph
- Remove download support for superhtml on Linux (#194)
- Remove Windows OpenConsole.exe binary from source code tree
- Removed unused message_editor and notification_panel settings
- Remove project name setting (#278)

### Fixed

- Don't terminate connection on ignorable LSP messages
- Ensure that buffer is up to date after undo (zed#51037)
- Fix edited multibuffers not saving on focus change when `autosave` setting is set to `on_focus_change`
- Fix environment variables failing to load when `nu` is the login shell
- Fix gitignore trying to watch the home directory
- Fix markdown block quote continuation highlighting (zed#51465)
- Fix missing icon in GNOME overview. (#198) by @topas-rec
- Fix opening closed projects randomly when Gram restarts (zed#50961)
- Fix rewrapping with an empty selection (zed#51742)
- Fix scrollbar breaking when UI font size changes (zed#45099)
- Fix unknown capture warning for TOML files
- Fix watcher cleanup for recreated directories (zed#50412)
- Handle FS rescan events (zed#51208)
- Handle symlinked settings files in watcher
- Include file size in DiskState to fix stale buffer reload (zed#48691)
- Open named in-memory databases as SQLite URIs (zed#50967)
- Fix crash with Unicode chars whose lowercase expands to multiple codepoints (zed#52989)
- Fix an ordering problem that led to invalid edits in display map sync (zed#52930)
- Fix out-of-bounds indexing when diff bases contain CRLF (zed#52605)
- Fix alt keybindings capturing altgr input (Mac) (#206)
- Minor fixes to the build scripts to allow a clean build on Chimera Linux (#205) by @jmc
- Fix dollar sign rendering in Markdown preview (zed#50440)
- Prevent stack overflows in Markdown parsing (zed#51637)
- Fix Markdown preview not re-rendering on external change (zed#50583)
- Fix table wrapping behaviour in Markdown preview (zed#50839)
- Make non-zero number fields editable in settings (#232)
- fix: .gitattributes stats, update langs case by @koru (#257)
- Fix file rename on FUSE-based filesystems (#269) by @mxw
- Fix history navigation in multibuffers (#275)
- Fix incorrect window size on X11 (#259)
- Improved Supertab completion
- Fix application menus hover behavior (#276) by @sir_nacnud
- Fix tab completion in file finder (#270)
- Don't install compilation artifacts when installing extension (#273)
- Compile extension in release mode (#273)
- Update vscode-eslint to 3.0.24 and fix ESLint 8-10 (#227) (zed#52886)
- Fix git graph text getting squashed (#272)

### Changed

- Modified default settings to be more sensible (#184)
- Update to Rust 1.95
- Update and clean up dependencies
- Enable panel icons if default and icon theme is changed (#172)
- Add Arch package build
- Upgrade cosmic-text to 0.18.2 (#199) by @esotericwitch
- Use SSH nicknames in display names (zed#53103)
- Bump tree-sitter for fix to wasm loading of grammars w/ reserved words (zed#52856)
- Update .gitattributes (#226) by @ivankopylov6603
- Make status_bar.show a non-experimental setting
- Rework vim/helix mode indicator
- Added screenshot to the README (#263) by @arisunz
- Updated README (#279) by @koru

## [1.2.1] - 2026-03-26

### Fixed

- Make sure to update Cargo.lock before tagging
- docs: Adopt the Zig project code of conduct policy on LLM use
- docs: Add toad to README
- docs: Add explanation for the name to README (#155)
- docs: Fix markdown link in README

## [1.2.0] - 2026-03-24

### Added

- Placeholder in empty pane when no file is open (#148)

### Fixed

- Use new icon format on Mac OS Tahoe (#146) by @kramo
- Fix: Use CompositeAlphaMode::Auto as fallback instead of ::Opaque (#149)
- Fix: handle wgpu surface recreation better (#149)
- Fix: Check for wasm-[component]-ld when detecting clang for WASI (#64)
- docs: Update installation instructions for Linux (#159) by @theDoctor
- docs: Fix a bunch of broken documentation links

### Changed

- Don't load defaults for Minimal base keymap (#151) by @tjk
- Increase contrast of selected item in Prompt window (#156) by @tjk
- Remove (untested) Snap support from script folder

## [1.1.0] - 2026-03-21

### Added

- Add AUR installation instructions to README (#38) by @nerdyslacker
- Add Gleam theme by Danielle Maywood
- Add Minimal base keymap for use with vim/helix keys (#144) by @tjk
- Add XML language server lemminx (#121) by @theDoctor
- Add XML treesitter support (#116) by @theDoctor
- Add appimage build option to bundle-linux (#114) by @theDoctor
- Add bash LSP (#132) by @theDoctor
- Add binary aur install instructions to README (#53) by @bananas
- Add default empty rustfmt config (#92) by @fzzr
- Add helix :reflow command to vim/helix modes
- Add new app icons by @kramo (#49) (#134)
- Add perl-Time-Piece to linux script (#1) by @LHolten
- Add remove trailing whitespace action (#81)
- Add subpixel text rendering, switch from blade to wgpu (#103) by @selfisekai
- Add zlib-ng-compat-static as dependency for Fedora build (#62) by @voidedgin
- Build and package for windows (#74) by @sparx
- Built in language support for Nix (#59)
- Built in language support for OpenTofu (#33) by @theDoctor
- Improved feedback when extension installation fails (#37)
- Options to adjust client side decoration rounding/shadow (#20)
- Support /path/to/file.txt(ln, col) style paths in terminal (#75)

### Fixed

- Bind vim/helix keys even if base keymap is None (#117)
- Cleanup remaining screen-capture code (#80) by @selfisekai
- Don't install WASI SDK if compatible clang is found (#41)
- Don't panic on failing to send an error notification (#136) by @selfisekai
- Enable the uninstall extension button (#32)
- Extend icon theme docs with enable settings (#6) by @Petrosz007
- Fetch newest release automatically (#83) by @scadu
- Fix LSP github download logic for pre-release (#67) by @Petrosz007
- Fix Supertab not performing word completion
- Fix `block_comment` and `documentation_comment` for Rust (#24) by @fzzr
- Fix download of tofu-ls for opentofu lsp support (#122)
- Fix empty project panel "or" divider (#109)
- Fix extensions being installed to tmp dir (#26)
- Fix menubar keyboard navigation (#119) by @aylamz
- Fix superhtml LSP (#56) by @Petrosz007
- Fix the lua-language-server asset name when downloading from github (#78)
- Fixed package name for cmake for Gentoo packages (#28) by @stepanov
- Fixes compilation issue on aarch64-linux (#61) by @voidedgin
- Make UI and Buffer Font match. (#69) by @voidedgin
- Modify single instance port numbers to not clash with Zed (#10)
- Remove more old sign-in code (#123) by @tjk
- Remove unused credentials code (#110)
- Set correct target for extension grammar (#84) by @selfisekai
- Use system clang, if it supports the wasm target (#64) by @selfisekai
- docs: Documentation around installing extensions. (#23) by @edwardloveall
- docs: Fix a few documentation links (#125) by @tjk
- docs: Fixed typos in README (#19) by @GulfSugar
- docs: Highlight that Rust in required to install some extensions (#34) by @ash-sykes
- docs: Remove mention of old feedback window (#126) by @tjk
- docs: Wasm is not an acronym

### Changed

- Move GLSL extension into [separate
  repo](https://codeberg.org/GramEditor/glsl-extension)
- Move Protobuf extension into [separate
  repo](https://codeberg.org/GramEditor/proto-extension)
- Allow for CARGO_TARGET_DIR in install.sh (#118) by @tjk
- Bump crash-handler to 0.7 (#39) by @selfisekai
- Update wild to 0.8.0 (#63) by @voidedgin
- (see [codeberg.org/GramEditor/gram](https://codeberg.org/GramEditor/gram) for
  complete list of changes)

## [1.0.0] - 2026-03-01

### Added

- First release
- Website: <https://gram.liten.app>

<!-- prettier-ignore-end -->
