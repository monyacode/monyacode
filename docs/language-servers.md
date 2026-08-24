# Language Servers

MonyaCode relies on _Language Servers_ to provide semantic information about
different programming languages. Each language has its own set of language
servers. A language server is a separate program which MonyaCode communicates with
using the _Language Server Protocol_. Many code editors speak the LSP these days
and so different editors can provide advanced language features like
autocompletion without having to implement them independently.

If MonyaCode knows about a particular language, either natively or via an extension,
it can use any language servers it finds already installed on the system.

MonyaCode can also download and install language servers. The Zed editor will do this
without asking, but MonyaCode tries not to do anything like downloading binaries from
the internet without explicit permission to do so.

Once MonyaCode has failed to launch a language server, an error will be reported and
the bot icon in the lower left of the editor window will have a red indicator.
Click that icon and then choose `Configure Servers`. Alternatively use the
Command Palette to run the {#action lsp::OpenLanguageServerConfig} command.

In this view, you can see all the loaded language servers and choose whether to
install any missing language servers yourself or let the editor try to install
them for you.
