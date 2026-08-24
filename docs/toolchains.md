# Toolchains

The toolchain selection UI is used to pick from a set of tools when working with
a given language in a current project.

Imagine that you are working in a Python project, which has virtual environments
that encapsulate a set of dependencies of your project along with a suitable
interpreter to run it with. The language server has to know which virtual
environment you are working with, as it uses the virtual environment to resolve
dependencies and parse the code in the project.

The toolchain selector UI is used to pick the correct virtual environment for
your project.

It's also possible to select different toolchains for different sub-projects
within a MonyaCode project. The exact definition of a sub-project depends on the
language used.

In [remote projects](./remote-development.md), use the toolchain selector to
control the active toolchain on the remote host.

## Why are toolchains necessary?

Language servers need to use the correct toolchain / virtual environment to
function properly. If it cannot resolve dependencies, functionality like "Go to
definition" or code completion may be unavailable.

If the toolchain provides an "activation script", the integrated terminal panel
will invoke the script automatically so that the shell environment is set up
correctly.

This also applies to [tasks](./tasks.md). Tasks behave as if they were launched
from a new terminal tab, so any activation script will be called before
executing a task.

## Selecting toolchains

The active toolchain (if any) is displayed in the status bar, on the right hand
side of the editor window. Clicking it open the toolchain selector.
Alternatively, run the command {#action toolchain::Select} from the command
palette.

The editor will automatically try to infer a set of toolchains to choose from. A
default will also be selected on a best-effort basis when opening a project for
the first time.

Toolchain selection applies to the current subproject. This might be the whole
project, or in the case of a monorepo with multiple subprojects it may apply to
the current subproject only.

## Adding toolchains manually

If the automatic detection fails to discover all toolchains you can add
additional toolchains manually using the "Add toolchain" button in the toolchain
selector UI.

Provide a path to the toolchain and give it a descriptive name.
