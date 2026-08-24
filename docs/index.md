# Documentation

MonyaCode is a source code editor and an IDE. It has a lot of features but is also
very much a work in progress. It is a community project, although the community
right now consists of one grumpy toad in its spare time and a small but
dedicated group of contributors (much love to them). The project hopes to move
slowly but steadily in the direction of stability, reliability and respect.

It has a lot of things going for it:

- It mostly works today. I use it every day to write code.
- Decent performance.
- Cross-platform support for Mac, Linux, FreeBSD and Windows.
- Remote development over SSH.
- Integrated debugger support via the `DAP` protocol.
- Jupyter Notebook support (REPL mode).
- Built in git support.
- Pretty decent Vim and Helix modal editing modes.
- Partial/limited support for Zed Wasm extensions.
- No AI integration or monetization scheme.
- No telemetry or proprietary server component.
- No video or audio chat.
- No involuntary auto updates.
- Tries not to install anything without explicit permission.
- Integrated documentation.
- Built in support for a lot of languages and language servers.
- Lots of builtin themes.

This project did not start with a vision of the perfect editor, but as a
reaction against what was and still is happening to the world of software
development, the world at large and to editors like VS Code, Zed and Vim. Its
core philosophy is that writing code by hand with care is good and righteous,
and aims to be a good tool for doing just that.

MonyaCode started as a **hard fork** of the Zed editor. For more details on the
background and motivation behind this fork, read the
[Mission Statement](./mission.md).

### Migrating

- From [Zed](./migrate/zed.md)
- From [VS Code](./migrate/vs-code.md)

## Features

- [Debugger](./debugger.md): Integrated support for DAP, the debugger adapter
  protocol.
- [Remote Development](./remote-development.md): Connect to remote servers via
  SSH and edit as if working on a local project.
- [Extensions](./extensions.md): Add support for additional languages, themes
  and icons using the extension system.
- [Supported Languages](./languages.md)
- [Language Servers](./language-servers.md): MonyaCode relies on language servers for
  providing advanced semantic functionality for various programming languages.

## Development

- [Development](./development.md)
  - [macOS](./development/macos.md)
  - [Linux](./development/linux.md)
  - [Windows](./development/windows.md)
  - [FreeBSD](./development/freebsd.md)
  - [Using Debuggers](./development/debuggers.md)
  - [Glossary](./development/glossary.md)
- [Debugging Crashes](./development/debugging-crashes.md)

## Configuration

- [Configuring MonyaCode](./configuring-monyacode.md)
- [Configuring Languages](./configuring-languages.md)
  - [Toolchains](./toolchains.md)
- [Key bindings](./key-bindings.md)
  - [All Actions](./all-actions.md)
- [Snippets](./snippets.md)
- [Themes](./themes.md)
- [Icon Themes](./icon-themes.md)
- [Visual Customization](./visual-customization.md)
- [Vim Mode](./vim.md)
- [Helix Mode](./helix.md)
- [SuperTab](./supertab.md)

## Using MonyaCode

- [Multibuffers](./multibuffers.md)
- [Command Palette](./command-palette.md)
- [Command-line Interface](./command-line-interface.md)
- [Outline Panel](./outline-panel.md)
- [Code Completions](./completions.md)
- [Git](./git.md)
- [Debugger](./debugger.md)
- [Diagnostics](./diagnostics.md)
- [Tasks](./tasks.md)
- [Tab Switcher](./tab-switcher.md)
- [Remote Development](./remote-development.md)
- [Environment Variables](./environment.md)
- [REPL](./repl.md)

## Platform Support

- [Windows](./windows.md)
- [Linux](./linux.md)

## Handling Problems

- [Troubleshooting](./troubleshooting.md)
- [Uninstall](./uninstall.md)

## Extensions

> **NOTE:** The Zed extension system relies on a closed-source server component,
> which is stripped from MonyaCode. Instead, all extensions have to be built from
> source. Currently, there is no extension registry so the extensions have to be
> installed either via the suggestion popups or an URL and Wasm extensions need
> rustup installed in order to compile.

- [Overview](./extensions.md)
- [Installing Extensions](./extensions/installing-extensions.md)
- [Developing Extensions](./extensions/developing-extensions.md)
- [Extension Capabilities](./extensions/capabilities.md)
- [Language Extensions](./extensions/languages.md)
- [Debugger Extensions](./extensions/debugger-extensions.md)
- [Theme Extensions](./extensions/themes.md)
- [Icon Theme Extensions](./extensions/icon-themes.md)

## Integrations and related tools

There are some related projects mostly around making MonyaCode available in various
package managers and Linux distributions. This list is not complete, if you know
of any packaging effort, MonyaCode-specific extensions or anything like that, feel
free to submit a PR at <https://github.com/monyacode/monyacode>.

- **Homebrew (Mac):** <https://formulae.brew.sh/cask/monyacode>
- **Arch Linux:** <https://archlinux.org/packages/extra/x86_64/monyacode>
- **Arch Linux (AUR):** <https://aur.archlinux.org/packages/monyacode-git>
- **Alpine Linux:**
  <https://pkgs.alpinelinux.org/package/edge/testing/x86_64/monyacode>
- **Gentoo Linux:** <https://github.com/monyacode/monyacode-gentoo>
- **Raycast (Mac):** <https://www.raycast.com/justyt65/monyacode>
- **Chimera Linux (WIP):** <https://github.com/chimera-linux/cports/pull/5506>

## Legal note on accepting contributions

If you have previously installed Zed and agreed to their license agreement, you
may be legally prevented from contributing to MonyaCode despite the open source
license of the code. I am not a lawyer and I suspect that the license that they
use would not hold up at least in European court, but I don't know. For that
exact reason, I never agreed to their license. This is the main reason this fork
even exists.

If you do want to contribute patches, you will have to accept full
responsibility for ensuring and warranting that you are legally allowed to do
so.

## You are the community

MonyaCode is proudly open source, in spirit, not just in words. That said, we have
strong opinions about what we want to include in the editor. For example, the
main reason for this fork from Zed is to remove certain "features" that we
disagree with, morally. However, you are of course free to make it your own in
any way you see fit.

There is no official discord or reddit community, but there is an XMPP chat for
MonyaCode at [monyacode@rooms.slidge.im][xmpp-link]. Any XMPP client should be able to
connect, and there is a [basic web UI][xmpp-webui] available. There are chat
logs available in an [online archive][xmpp-archive] as well.

[xmpp-link]: xmpp:monyacode@rooms.slidge.im?join
[xmpp-webui]: https://slidge.im/monyacode/#/guest?join=monyacode@rooms.slidge.im
[xmpp-archive]: https://rooms.slidge.im:5281/muc_log/monyacode/

## Strict No AI/LLM Policy

No more AI. I used to have a milder version of this statement here before, which
I wrote early on when I wasn't really aware of "vibe-coding" as such and was
mostly annoyed purely at the chatbox / autocomplete version of AI. That was bad
enough, but I really am not a fan of what that has become (in March 2026 when I
am writing this). I have copied this policy from the [Zig language project Code
of Conduct][zig-coc]:

> No LLMs for issues.
>
> No LLMs for pull requests.
>
> No LLMs for comments on the bug tracker, including translation. English is
> encouraged, but not required. You are welcome to post in your native language
> and rely on others to have their own translation tools of choice to interpret
> your words.

The Zed code base contains a lot of AI-generated code. It doesn't need a single
line more.

[zig-coc]: https://ziglang.org/code-of-conduct/
