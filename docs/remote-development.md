# Remote Development

MonyaCode supports remote editing over SSH, where you run the editor on your main
machine but open and edit projects on remote machines.

To make this work, MonyaCode uses a remote process called `remote-server`, which
needs to be installed on the machine that the editor should connect to. The
editor will SSH to the remote machine and start the remote server which then
takes care of opening, reading and writing files as well as running language
servers.

The remote server communicates with the editor over SSH and GRPC.

## Overview

Remote development requires two computers, your local machine that runs the UI
and the remote server which runs a small server. The two communicate over SSH,
so you will need to be able to SSH from your local machine into the remote
server to use this feature.

On your local machine, MonyaCode runs its UI, uses
Tree-sitter to parse and syntax-highlight code, and store unsaved changes and
recent projects. The source code, language servers, tasks, and the terminal all
run on the remote server.

## Setup

1. Install the editor on your main machine.

2. Build and copy the remote editor process to your development machine.

   Either get a pre-built copy of the remote server from the
   [Codeberg releases](https://github.com/monyacode/monyacode/releases) page, or
   build it from source.

   If you download the pre-built release, make sure to get a version that
   matches your remote server. If you are running the editor on a Mac but want
   to connect to a Linux machine, you will need the Linux version of the
   remote-server.

   Also make sure to get the same version of the remote server as your editor.

   Decompress and copy the server to
   `~/.monyacode_server/monyacode-remote-server-{RELEASE_CHANNEL}-{VERSION}`.

   For example:

   ```sh
   mkdir -p ~/.monyacode_server
   curl -L -o - \
     https://github.com/monyacode/monyacode/releases/download/1.1.0/monyacode-remote-server-linux-x86_64-1.1.0.gz \
     | gunzip > ~/.monyacode_server/monyacode-remote-server-stable-1.1.0
   chmod +x ~/.monyacode_server/monyacode-remote-server-stable-1.1.0
   ```

3. Use {#kb projects::OpenRemote} to open the "Remote Projects" dialog.

4. Click "Connect New Server" and enter the command you use to SSH into the
   server. See [Supported SSH options](#supported-ssh-options) for options you
   can pass.

5. Your local machine will attempt to connect to the remote server using the
   `ssh` binary on your path. Assuming the connection is successful and a
   compatible server binary is found, the editor will start up and start
   communicating with the remote server.

6. Once the remote server is running, you will be prompted to choose a path to
   open on the remote server.

   > **Note:** The remote server does not currently handle opening very large
   > directories (for example, `/` or `~` that may have >100,000 files) very
   > well.

   On a Mac, the editor may fail to connect the first time you try, and you will
   be presented with a permissions dialog to allow MonyaCode to connect to the
   network. You'll need to accept this and then try again.

### Connect from the command line

To open the editor and connect to a remote project directly from the terminal,
use a command like `monyacode ssh://[<user>@]<host>[:<port>]/<path-to-project>`.

It is currently not possible to pass any other arguments to SSH from the
commandline. A workaround here is to set other options you need in your SSH
config file. See the SSH documentation (`man ssh_config`) for more details.

To hotlink into an SSH project, use a link of the format:
`monyacode://ssh/[<user>@]<host>[:<port>]/<path>`.

## Supported platforms

The remote machine must be able to run the remote server process, and so you
need to compile the remote server process for the target platform (cross
compilation).

The following platforms should work:

- macOS Catalina or later (Intel or Apple Silicon)
- Linux (x86_64 or arm64)
- Windows is not yet supported as a remote server, but Windows can be used as a
  local machine to connect to remote servers. Note that Windows support in is
  currently not well tested in general.

## Configuration

The list of remote servers is stored in your settings file {#kb
monyacode::OpenSettings}. You can edit this list using the Remote Projects dialog
{#kb projects::OpenRemote}, which provides some robustness - for example it
checks that the connection can be established before writing it to the settings
file.

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "projects": [{ "paths": ["~/code/monyacode/monyacode"] }],
    },
  ],
}
```

MonyaCode shells out to the `ssh` on your path, and so it will inherit any
configuration you have in `~/.ssh/config` for the given host. That said, if you
need to override anything you can configure the following additional options on
each connection:

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "projects": [{ "paths": ["~/code/monyacode/monyacode"] }],
      // any argument to pass to the ssh master process
      "args": ["-i", "~/.ssh/work_id_file"],
      "port": 22, // defaults to 22
      // defaults to your username on your local machine
      "username": "me",
    },
  ],
}
```

You can also set a nickname for the remote server:

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "projects": [{ "paths": ["~/code/monyacode/monyacode"] }],
      // Shown in the UI to help distinguish multiple hosts.
      "nickname": "lil-linux",
    },
  ],
}
```

If you use the command line to open a connection to a host by doing
`monyacode ssh://192.168.1.10/~/.vimrc`, then extra options are read from your
settings file by finding the first connection that matches the
host/username/port of the URL on the command line.

Additionally it's worth noting that while you can pass a password on the command
line `monyacode ssh://user:password@host/~`, we do not support writing a password to
your settings file. If you're connecting repeatedly to the same host, you should
configure key-based authentication.

## Remote Development on Windows (SSH)

MonyaCode on Windows supports SSH remoting and will prompt for credentials when
needed.

If you encounter authentication issues, confirm that your SSH key agent is
running (e.g., ssh-agent or your Git client's agent) and that ssh.exe is on
PATH.

### Troubleshooting SSH on Windows

When prompted for credentials, use the graphical askpass dialog. If it doesn't
appear, check for credential manager conflicts and that GUI prompts aren't
blocked by your terminal.

## WSL Support

MonyaCode supports opening folders inside of WSL natively on Windows.

### Opening a local folder in WSL

To open a local folder inside a WSL container, use the `projects: open in wsl`
action and select the folder you want to open. You will be presented with a list
of available WSL distributions to open the folder in.

### Opening a folder already in WSL

To open a folder that's already located inside of a WSL container, use the
`projects: open wsl` action and select the WSL distribution. The distribution
will be added to the `Remote Projects` window where you will be able to open the
folder.

## Port forwarding

If you'd like to be able to connect to ports on your remote server from your
local machine, you can configure port forwarding in your settings file. This is
particularly useful for developing websites so you can load the site in your
browser while working.

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "port_forwards": [{ "local_port": 8080, "remote_port": 80 }],
    },
  ],
}
```

This will cause requests from your local machine to `localhost:8080` to be
forwarded to the remote machine's port 80. Under the hood this uses the `-L`
argument to ssh.

By default these ports are bound to localhost, so other computers in the same
network as your development machine cannot access them. You can set the
local_host to bind to a different interface, for example, 0.0.0.0 will bind to
all local interfaces.

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "port_forwards": [
        {
          "local_port": 8080,
          "remote_port": 80,
          "local_host": "0.0.0.0",
        },
      ],
    },
  ],
}
```

These ports also default to the `localhost` interface on the remote host. If you
need to change this, you can also set the remote host:

```json
{
  "ssh_connections": [
    {
      "host": "192.168.1.10",
      "port_forwards": [
        {
          "local_port": 8080,
          "remote_port": 80,
          "remote_host": "docker-host",
        },
      ],
    },
  ],
}
```

## MonyaCode settings

When opening a remote project there are three relevant settings locations:

- The local MonyaCode settings (in `~/.monyacode/settings.json` on macOS or
  `~/.config/monyacode/settings.json` on Linux) on your local machine.
- The server MonyaCode settings (in the same place) on the remote server.
- The project settings (in `.monyacode/settings.json` or `.editorconfig` of your
  project)

Both the local MonyaCode and the server MonyaCode read the project settings, but they are
not aware of the other's main `settings.json`.

Which settings file you should use depends on the kind of setting you want to
make:

- Project settings should be used for things that affect the project:
  indentation settings, which formatter / language server to use, etc.
- Server settings should be used for things that affect the server: paths to
  language servers, proxy settings, etc.
- Local settings should be used for things that affect the UI: font size, etc.

In addition any extensions you have installed locally will be propagated to the
remote server. This means that language servers, etc. will run correctly.

## Proxy Configuration

The remote server will not use your local machine's proxy configuration because
they may be under different network policies. If your remote server requires a
proxy to access the internet, you must configure it on the remote server itself.

In most cases, your remote server will already have proxy environment variables
configured. MonyaCode will automatically use them when downloading language servers,
communicating with LLM models, etc.

If needed, you can set these environment variables in the server's shell
configuration (e.g., `~/.bashrc`):

```bash
export http_proxy="http://proxy.example.com:8080"
export https_proxy="http://proxy.example.com:8080"
export no_proxy="localhost,127.0.0.1"
```

Alternatively, you can configure the proxy in the remote machine's
`~/.config/monyacode/settings.json` (Linux) or `~/.monyacode/settings.json` (macOS):

```json
{
  "proxy": "http://proxy.example.com:8080"
}
```

See the [proxy documentation](./configuring-monyacode.md#network-proxy) for supported
proxy types and additional configuration options.

## Initializing the remote server

Once you provide the SSH options, MonyaCode shells out to `ssh` on your local machine
to create a ControlMaster connection with the options you provide.

Any prompts that SSH needs will be shown in the UI, so you can verify host keys,
type key passwords, etc.

Once the master connection is established, MonyaCode will check to see if the remote
server binary is present in `~/.monyacode_server` on the remote, and that its version
matches the current version of MonyaCode that you're using.

To build the remote server binary, run `cargo build -p remote_server --release`.
Upload it to `~/.monyacode_server/monyacode-remote-server-{RELEASE_CHANNEL}-{VERSION}` on
the server, for example `~/.monyacode_server/monyacode-remote-server-stable-0.181.6`. The
version must exactly match the version of MonyaCode itself you are using.

## Maintaining the SSH connection

Once the server is initialized. MonyaCode will create new SSH connections (reusing
the existing ControlMaster) to run the remote development server.

Each connection tries to run the development server in proxy mode. This mode
will start the daemon if it is not running, and reconnect to it if it is. This
way when your connection drops and is restarted, you can continue to work
without interruption.

In the case that reconnecting fails, the daemon will not be re-used. That said,
unsaved changes are by default persisted locally, so that you do not lose work.
You can always reconnect to the project at a later date and MonyaCode will restore
unsaved changes.

If you are struggling with connection issues, you should be able to see more
information in the MonyaCode log `cmd-shift-p Open Log`.

## Supported SSH Options

Under the hood, MonyaCode shells out to the `ssh` binary to connect to the remote
server. We create one SSH control master per project, and then use that to
multiplex SSH connections for the MonyaCode protocol itself, any terminals you open
and tasks you run. We read settings from your SSH config file, but if you want
to specify additional options to the SSH control master you can configure MonyaCode
to set them.

When typing in the "Connect New Server" dialog, you can use bash-style quoting
to pass options containing a space. Once you have created a server it will be
added to the `"ssh_connections": []` array in your settings file. You can edit
the settings file directly to make changes to SSH connections.

Supported options:

- `-p` / `-l` - these are equivalent to passing the port and the username in the
  host string.
- `-L` / `-R` for port forwarding
- `-i` - to use a specific key file
- `-o` - to set custom options
- `-J` / `-w` - to proxy the SSH connection
- `-F` for specifying an `ssh_config`
- And also... `-4`, `-6`, `-A`, `-B`, `-C`, `-D`, `-I`, `-K`, `-P`, `-X`, `-Y`,
  `-a`, `-b`, `-c`, `-i`, `-k`, `-l`, `-m`, `-o`, `-p`, `-w`, `-x`, `-y`

Note that we deliberately disallow some options (for example `-t` or `-T`) that
MonyaCode will set for you.

## Known Limitations

- You can't open files from the remote Terminal by typing the `monyacode` command.
