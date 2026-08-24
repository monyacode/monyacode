# OCaml

OCaml support is available through the
[OCaml extension](https://github.com/zed-extensions/ocaml).

- Tree-sitter:
  [tree-sitter/tree-sitter-ocaml](https://github.com/tree-sitter/tree-sitter-ocaml)
- Language Server: [ocaml/ocaml-lsp](https://github.com/ocaml/ocaml-lsp)

## Setup Instructions

If you have the development environment already setup, skip to
[Launching MonyaCode](#launching-monyacode)

### Using Opam

Opam is the official package manager for OCaml and is highly recommended for
getting started with OCaml. To get started using Opam, please follow the
instructions provided [here](https://ocaml.org/install).

Proceed once opam is installed and you have setup a switch with your development
environment as per the instructions.

### Launching MonyaCode

Verify that the `ocamllsp` command line tool for LSP is installed:

```sh
ocamllsp --help
```

If the command is missing or fails to display the help text, revisit the
installation instructions for `ocamllsp` and ensure it's properly installed.

Note: Given how the OCaml package manager works, MonyaCode needs to run from the
terminal, so make sure to install the MonyaCode cli.

From a terminal, navigate to an OCaml project and run

```sh
monyacode .
```

MonyaCode should now be running with OCaml support.
