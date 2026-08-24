# Git

There is integrated support for working with git and git forges in the code
editor.

Here's an overview of all currently supported features:

- Committing
- Staging, pushing, pulling, and fetching
- Project Diff: A multibuffer view of all changes
- Diff indicators in buffers and editor scrollbars
- Inline diff toggle and reverts in the editor for unstaged changes
- Git status in the Project Panel
- Branch creating and switching
- Git blame viewing
- Git stash pop, apply, drop and view

## Git Panel

The Git Panel provides a birds-eye view of the state of the working tree and
staging area.

Open the Git Panel using {#action git_panel::ToggleFocus}, or by clicking the
Git icon in the status bar.

The Git Panel shows the state of the project: Which repository and branch are
active, what files have changed and the current staging state of each file.

MonyaCode monitors the project repository, so any changes made within or outside the
editor are instantly visible in the git panel.

### Configuration

Configure how the editor hard wraps commit messages with the
`preferred-line-length` setting of the "Git Commit" language. The default is
`72`, but it can be set to any number of characters, `0` or more.

The Git Panel also allows configuring the `soft_wrap` setting to adjust how
commit messages display while typing them in the Git Panel. The default setting
is `editor_width`, however, `none`, `preferred_line_length`, and `bounded` are
also options.

#### Example

```json
"languages": {
  "Git Commit": {
    "soft_wrap": "editor_width",
    "preferred_line_length": 72
  },
}
```

## Project Diff

See all of the current changes by opening the Project Diff ({#kb git::Diff}),
accessible via the {#action git::Diff} action in the Command Palette or the Git
Panel.

All of the changes displayed in the Project Diff behave the same as any other
multibuffer: they are all editable excerpts of files.

Stage or unstage each hunk as well as a whole file by hitting the buttons on the
tab bar or their corresponding keybindings.

## Fetch, push, and pull

Fetch, push, or pull from the Git repository in MonyaCode via the buttons available
on the Git Panel or via the Command Palette actions: {#action git::Fetch},
{#action git::Push}, and {#action git::Pull}.

## Staging Workflow

MonyaCode has two primary staging workflows, using either the Project Diff or the
panel directly.

### Using the Project Diff

In the Project Diff view it's possible to focus on each hunk and stage them
individually by clicking on the tab bar buttons or via the keybindings {#action
git::StageAndNext} ({#kb git::StageAndNext}).

Similarly, stage all hunks at the same time with the {#action git::StageAll}
({#kb git::StageAll}) keybinding and then immediately commit with {#action
git::Commit} ({#kb git::Commit}).

### Using the Git Panel

From the panel, simply type a commit message and hit the commit button, or
{#action git::Commit}. This will automatically stage all tracked files
(indicated by a `[·]` in the entry's checkbox) and commit them.

Entries can be staged using each individual entry's checkbox. All changes can be
staged using the button at the top of the panel, or {#action git::StageAll}.

## Committing

There are two ways to commit changes from within the editor:

1. Using the button at the bottom of the Git Panel. Hitting {#kb git::Commit}
   immediately commits all of your staged changes.
2. Invoke the action {#action git::ExpandCommitEditor} or use the keyboard
   shortcut {#kb git::ExpandCommitEditor} while focused on the Git Panel commit
   text area.

### Undo a commit

As soon as you commit in MonyaCode, in the Git Panel, you'll see a bar right under
the commit textarea which will show the most recently submitted commit.

In there, you can use the "Uncommit" button, which will run a command similar to
`git reset HEADˆ --soft`.

### Commit Line Length

By default, the commit line length is set to `72`. It can be configured in
`settings.json`.

See `preferred-line-length` in the [Configuration](#configuration) section.

## Stashing

Git stash allows you to temporarily save your uncommitted changes and return to
a clean working directory. This is useful to quickly switch branches or pull
updates without committing incomplete work.

### Creating Stashes

To stash all current changes, use the {#action git::StashAll} action. This will
save both staged and unstaged changes to a new stash entry and set up a clean
working directory.

### Managing Stashes

MonyaCode provides a comprehensive stash picker accessible via {#action
git::ViewStash}. From the stash picker, you can:

- **View stash list**: Browse all your saved stashes with their descriptions and
  timestamps
- **Open diffs**: See exactly what changes are stored in each stash
- **Apply stashes**: Apply stash changes to your working directory while keeping
  the stash entry
- **Pop stashes**: Apply stash changes and remove the stash entry from the list
- **Drop stashes**: Delete unwanted stash entries without applying them

### Quick Stash Operations

For faster workflows, MonyaCode provides direct actions to work with the most recent
stash:

- **Apply latest stash**: Use {#action git::StashApply} to apply the most recent
  stash without removing it
- **Pop latest stash**: Use {#action git::StashPop} to apply and remove the most
  recent stash

### Stash Diff View

When viewing a specific stash in the diff view, you have additional options
available through the interface:

- Apply the current stash to your working directory
- Pop the current stash (apply and remove)
- Remove the stash without applying changes

To open the stash diff view, select a stash from the stash picker and use the
{#action stash_picker::ShowStashItem} ({#kb stash_picker::ShowStashItem})
keybinding.

## Git Integrations

MonyaCode integrates with popular Git hosting services to ensure that Git commit
hashes and references to Issues, Pull Requests, and Merge Requests become
clickable links.

MonyaCode currently supports links to the hosted versions of
[GitHub](https://github.com), [GitLab](https://gitlab.com),
[Bitbucket](https://bitbucket.org), [Sourcehut](https://sr.ht) and
[Codeberg](https://codeberg.org).

For self-hosted GitHub, GitLab, or Bitbucket instances, add them to the
`git_hosting_providers` setting so commit hashes and permalinks resolve to your
domain:

```json
{
  "git_hosting_providers": [
    {
      "provider": "gitlab",
      "name": "Corp GitLab",
      "base_url": "https://git.example.corp",
    },
  ],
}
```

## Copy Permalink

Create a permanent link to a code snippet on your Git hosting service.

These links are useful for sharing a specific line or range of lines in a file
at a specific commit. Trigger this action via the Command Palette (search for
`permalink`), by creating a
[custom key bindings](./key-bindings.md#custom-key-bindings) to the
`editor::CopyPermalinkToLine` or `editor::OpenPermalinkToLine` actions or by
simply right clicking and selecting `Copy Permalink` with line(s) selected in
your editor.

## Diff Hunk Keyboard Shortcuts

When viewing files with changes, MonyaCode displays diff hunks that can be expanded
or collapsed for detailed review:

- **Expand all diff hunks**: {#action editor::ExpandAllDiffHunks} ({#kb
  editor::ExpandAllDiffHunks})
- **Collapse all diff hunks**: Press `Escape` (bound to {#action
  editor::Cancel})
- **Toggle selected diff hunks**: {#action editor::ToggleSelectedDiffHunks}
  ({#kb editor::ToggleSelectedDiffHunks})
- **Navigate between hunks**: {#action editor::GoToHunk} and {#action
  editor::GoToPreviousHunk}

> **Tip:** The `Escape` key is the quickest way to collapse all expanded diff
> hunks and return to an overview of your changes.

## Action Reference

| Action                                    | Keybinding                            |
| ----------------------------------------- | ------------------------------------- |
| {#action git::Add}                        | {#kb git::Add}                        |
| {#action git::StageAll}                   | {#kb git::StageAll}                   |
| {#action git::UnstageAll}                 | {#kb git::UnstageAll}                 |
| {#action git::ToggleStaged}               | {#kb git::ToggleStaged}               |
| {#action git::StageAndNext}               | {#kb git::StageAndNext}               |
| {#action git::UnstageAndNext}             | {#kb git::UnstageAndNext}             |
| {#action git::Commit}                     | {#kb git::Commit}                     |
| {#action git::ExpandCommitEditor}         | {#kb git::ExpandCommitEditor}         |
| {#action git::Push}                       | {#kb git::Push}                       |
| {#action git::ForcePush}                  | {#kb git::ForcePush}                  |
| {#action git::Pull}                       | {#kb git::Pull}                       |
| {#action git::PullRebase}                 | {#kb git::PullRebase}                 |
| {#action git::Fetch}                      | {#kb git::Fetch}                      |
| {#action git::Diff}                       | {#kb git::Diff}                       |
| {#action git::Restore}                    | {#kb git::Restore}                    |
| {#action git::RestoreFile}                | {#kb git::RestoreFile}                |
| {#action git::Branch}                     | {#kb git::Branch}                     |
| {#action git::Switch}                     | {#kb git::Switch}                     |
| {#action git::CheckoutBranch}             | {#kb git::CheckoutBranch}             |
| {#action git::Blame}                      | {#kb git::Blame}                      |
| {#action git::StashAll}                   | {#kb git::StashAll}                   |
| {#action git::StashPop}                   | {#kb git::StashPop}                   |
| {#action git::StashApply}                 | {#kb git::StashApply}                 |
| {#action git::ViewStash}                  | {#kb git::ViewStash}                  |
| {#action editor::ToggleGitBlameInline}    | {#kb editor::ToggleGitBlameInline}    |
| {#action editor::ExpandAllDiffHunks}      | {#kb editor::ExpandAllDiffHunks}      |
| {#action editor::ToggleSelectedDiffHunks} | {#kb editor::ToggleSelectedDiffHunks} |

> Not all actions have default keybindings, but can be bound by
> [customizing your keymap](./key-bindings.md#user-keymaps).

## Git CLI Configuration

If you would like to also use MonyaCode for your
[git commit message editor](https://git-scm.com/book/en/v2/Customizing-Git-Git-Configuration#_core_editor)
when committing from the command line you can use `monyacode --wait`:

```sh
git config --global core.editor "monyacode --wait"
```

Or add the following to your shell environment (in `~/.zshrc`, `~/.bashrc`,
etc):

```sh
export GIT_EDITOR="monyacode --wait"
```
