# Contributing

The over-arching goal for now is to excise the MonyaCode codebase of all the shady
stuff so that only the good parts remain. First and foremost, that means
removing all AI integration and telemetry.

Unfortunately, this means that some genuinely useful functionality may be lost
in the process. Thus, this is the plan for this project:

- [x] Strip out the AI, the telemetry, the subscriptions, the auto installation.
- [x] Make the automatic installation of language servers optional and prompt
      the user before installing anything (letting them decline).
- [ ] Make everything that broke in the process work again.
- [ ] Replace anything that was lost with better, open and respectful
      alternatives.

AI will never return. Some form of peer-to-peer collaboration support would be
cool to have, though.

Automatic installation of pre-built extensions is gone. Is there a way to make
the editor more extensible without losing that? Maybe an option is to add
something like the extensions planned for Helix.

Another option is to drop the extensions entirely and just build everything into
the editor: Support as many languages and themes as possible out of the box.

If this sounds interesting to you, feel free to help out.

It should go without saying but AI-generated pull requests are strictly banned
from this project. This codebase is bloated and vibe-coded enough as it is, we
need to fix and mend and patch and not start adding to the mess as much as
possible.

The project policy on AI and LLM use is outlined in the
[Code of Conduct](./CODE_OF_CONDUCT.md).

## Bird's-eye view of the code base

The [glossary](docs/development/glossary.md) lists and explains some of the
structures and terms you will see throughout the codebase.

The editor is made up of several smaller crates. This is an incomplete list of
crates and their use.

- [`gpui`](/crates/gpui) is a GPU-accelerated UI framework which provides all of
  the building blocks for the UI. [Documentation](https://www.gpui.rs)
- [`editor`](/crates/editor) contains the core `Editor` type that drives both
  the code editor and all various input fields. It also handles a display layer
  for LSP features such as Inlay Hints or code completions.
- [`project`](/crates/project) manages files and navigation within the filetree.
  It is also the editor side of communication with LSP.
- [`workspace`](/crates/workspace) handles local state serialization and groups
  projects together.
- [`vim`](/crates/vim) is a thin implementation of Vim workflow over `editor`.
- [`lsp`](/crates/lsp) handles communication with external LSP server.
- [`language`](/crates/language) drives `editor`'s understanding of language -
  from providing a list of symbols to the syntax map.
- [`languages`](/crates/languages) contains language support for all of the
  builtin languages. The API for builtin languages is quite different from the
  zed extension API, so converting an extension into a builtin language is not
  always easy. Any language that is built in also needs a tree-sitter module
  which is up to date, so that excludes some languages from being supported
  without an extension (unless the tree-sitter module is updated).
- [`theme`](/crates/theme) defines the theme system and provides a default
  theme.
- [`ui`](/crates/ui) is a collection of UI components and common patterns used
  throughout the editor.
- [`cli`](/crates/cli) is the CLI crate which invokes the monyacode binary.
- [`monyacode`](/crates/monyacode) is where all things come together, and the `main` entry
  point for the project.

### Stores

There are a number of global registries referred to as stores in the codebase.
This is not a complete listing:

- [`settings_store`](/crates/settings/src/settings_store.rs) manages all of the
  settings and configuration.
- [`lsp_store`](/crates/project/src/lsp_store.rs) manages all language servers
  that have been loaded.
- [`worktree_store`](/crates/project/src/worktree_store.rs) contains information
  about all of the currently open worktrees.
