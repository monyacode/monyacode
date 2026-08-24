# MonyaCode on Linux

## Distro Packages

The preferred way to install is via adding a MonyaCode repository file and installing
from it. This provides the user with automatic updates once new packages are
released. See instructions below. Alternatively, MonyaCode provides prebuilt `.deb`
and `.rpm` packages as release assets which can be downloaded at
[MonyaCode releases](https://github.com/monyacode/monyacode/releases).

### Debian/Ubuntu

```sh
# Add the repository key
sudo curl https://codeberg.org/api/packages/MonyaCode/debian/repository.key -o /etc/apt/keyrings/forgejo-MonyaCode.asc
# Add the repository file
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/forgejo-MonyaCode.asc] https://codeberg.org/api/packages/MonyaCode/debian monyacode release" | sudo tee -a /etc/apt/sources.list.d/monyacode.list
# Update package cache and install
sudo apt update && sudo apt install monyacode
```

Requires Debian 12 (Bookworm)/Ubuntu 24.04 (noble) or later.

### Fedora/RHEL/Rocky/Alma

```sh
# Add repository file
sudo dnf config-manager addrepo --from-repofile="https://codeberg.org/api/packages/MonyaCode/rpm.repo"
# Install
sudo dnf install monyacode
```

**Note**: At the time of this writing RPM package signing does not work. In
order to be able to install MonyaCode you need to set `gpgcheck=0` inside
`/etc/yum.repos.d/rpm.repo`.

Requires Fedora 42/RHEL 10.1/Rocky 10.1/Alma 10.1 or later.

### OpenSUSE

```sh
sudo zypper addrepo https://codeberg.org/api/packages/MonyaCode/rpm.repo
sudo zypper install monyacode
```

**Note**: At the time of this writing RPM package signing does not work. In
order to be able to install MonyaCode you need to set `gpgcheck=0` inside
`/etc/yum.repos.d/rpm.repo`.

Requires Leap 16 (or Tumbleweed/Slowroll) or later.

### Arch

Arch Linux publishes official packages for MonyaCode in their
[extra](https://archlinux.org/packages/extra/x86_64/monyacode/) repository.

```sh
sudo pacman -S monyacode
```

Alternatively, to build and install a development snapshot from the latest Git
HEAD, build the VCS package from the aur:
[`monyacode-git`](https://aur.archlinux.org/packages/monyacode-git). If you install
packages from the AUR, it is your responsibility to verify their integrity
yourself.

### Alpine

Alpine Linux publishes official packages for MonyaCode in their
[testing](https://pkgs.alpinelinux.org/package/edge/testing/x86_64/monyacode)
repository. Follow the
[instructions](https://wiki.alpinelinux.org/wiki/Repositories#Using_testing_repository)
to enable the testing repository, then run

```sh
doas apk add monyacode@testing
```

### Gentoo GNU/Linux

Gentoo provided by MonyaCode's overlay.

```sh
# replace doas if you using sudo
doas eselect repository add monyacode git https://github.com/monyacode/monyacode-gentoo.git
doas emerge --sync monyacode
doas emerge -av app-editors/monyacode
```

**Note**: For more information about Gentoo package and installation ways go to
[MonyaCode's overlay repo](https://github.com/monyacode/monyacode-gentoo).

## Flatpak

MonyaCode provides a prebuilt flatpak as a release asset. It can be downloaded from
at [MonyaCode releases](https://github.com/monyacode/monyacode/releases) and installed
by running:

```sh
flatpak install /path/to/app.liten.MonyaCode-x86_64-${version}.flatpak
```

## From Tarball

If there is a tarball available for your architecture at the
[MonyaCode Codeberg](https://github.com/monyacode/monyacode/releases) repository, you
can follow these instructions:

1. Download the
   [install.sh](https://github.com/monyacode/monyacode/raw/branch/main/script/install.sh)
   script.
2. Run the script.

```sh
./install.sh
```

This will download latest release of MonyaCode and install MonyaCode to `$HOME/.local`. To
install system-wide, use the `--prefix PREFIX` argument:

```sh
./install.sh --prefix /usr/local ./monyacode-linux-x86_64-1.1.0.tar.gz
```

## From Source

MonyaCode is open source, and you can install from source. See
[developer notes](./development/linux.md) for instructions.

## Troubleshooting

### Graphics issues

#### MonyaCode fails to open windows

MonyaCode requires a GPU to run effectively. Under the hood, it uses
[Vulkan](https://www.vulkan.org/) to communicate with the GPU. If you are seeing
problems with performance or MonyaCode fails to load, it is possible that Vulkan is
the culprit.

If you see a notification saying
`MonyaCode failed to open a window: NoSupportedDeviceFound` this means that Vulkan
cannot find a compatible GPU. Try running
[vkcube](https://github.com/krh/vkcube) (usually available as part of the
`vulkaninfo` or `vulkan-tools` package on various distributions) to troubleshoot
where the issue is coming from like so:

```text
vkcube
```

> **_Note_**: Try running in both X11 and wayland modes by running
> `vkcube -m [x11|wayland]`. Some versions of `vkcube` use `vkcube` to run in
> X11 and `vkcube-wayland` to run in wayland.

This should output a line describing your current graphics setup and show a
rotating cube. If this does not work, you should be able to fix it by installing
Vulkan compatible GPU drivers, however in some cases there is no Vulkan support
yet.

You can find out which graphics card MonyaCode is using by looking in the MonyaCode log
(`~/.local/share/monyacode/logs/MonyaCode.log`) for `Using GPU: ...`.

If you see errors like `ERROR_INITIALIZATION_FAILED` or `GPU Crashed` or
`ERROR_SURFACE_LOST_KHR` then you may be able to work around this by installing
different drivers for your GPU, or by selecting a different GPU to run on. (See
[#14225](https://github.com/zed-industries/zed/issues/14225))

On some systems the file `/etc/prime-discrete` can be used to enforce the use of
a discrete GPU using [PRIME](https://wiki.archlinux.org/title/PRIME). Depending
on the details of your setup, you may need to change the contents of this file
to "on" (to force discrete graphics) or "off" (to force integrated graphics).

On others, you may be able to the environment variable `DRI_PRIME=1` when
running MonyaCode to force the use of the discrete GPU.

If you're using an AMD GPU and MonyaCode crashes when selecting long lines, try
setting the `MONYACODE_PATH_SAMPLE_COUNT=0` environment variable. (See
[#26143](https://github.com/zed-industries/zed/issues/26143))

If you're using an AMD GPU, you might get a 'Broken Pipe' error. Try using the
RADV or Mesa drivers. (See
[#13880](https://github.com/zed-industries/zed/issues/13880))

If you are using `amdvlk`, the default open-source AMD graphics driver, you may
find that MonyaCode consistently fails to launch. This is a known issue for some
users, for example on Omarchy (see issue
[#28851](https://github.com/zed-industries/zed/issues/28851)). To fix this, you
will need to use a different driver. We recommend removing the `amdvlk` and
`lib32-amdvlk` packages and installing `vulkan-radeon` instead (see issue
[#14141](https://github.com/zed-industries/zed/issues/14141)).

For more information, the
[Arch guide to Vulkan](https://wiki.archlinux.org/title/Vulkan) has some good
steps that translate well to most distributions.

#### Forcing MonyaCode to use a specific GPU

There are a few different ways to force MonyaCode to use a specific GPU:

##### Option A

You can use the `MONYACODE_DEVICE_ID={device_id}` environment variable to specify the
device ID of the GPU you wish to have MonyaCode use.

You can obtain the device ID of your GPU by running `lspci -nn | grep VGA` which
will output each GPU on one line like:

```sh
08:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA104 [GeForce RTX 3070] [10de:2484] (rev a1)
```

where the device ID here is `2484`. This value is in hexadecimal, so to force
MonyaCode to use this specific GPU you would set the environment variable like so:

```sh
MONYACODE_DEVICE_ID=0x2484 monyacode
```

Make sure to export the variable if you choose to define it globally in a
`.bashrc` or similar.

##### Option B

If you are using Mesa, you can run
`MESA_VK_DEVICE_SELECT=list monyacode --foreground` to get a list of available GPUs
and then export `MESA_VK_DEVICE_SELECT=xxxx:yyyy` to choose a specific device.
Furthermore, you can fallback to xwayland with an additional export of
`WAYLAND_DISPLAY=""`.

##### Option C

Using [vkdevicechooser](https://github.com/jiriks74/vkdevicechooser).

#### Generating debug reports

Passing the `--system-specs` flag to MonyaCode like

```sh
monyacode --system-specs
```

will print the system specs to the terminal.

The editor log is usually located at `~/.local/share/monyacode/logs/MonyaCode.log`.

To generate a clean log file for debugging graphics issues, run:

```sh
truncate -s 0 ~/.local/share/monyacode/logs/MonyaCode.log # Clear the log file
MONYACODE_LOG=wgpu=info monyacode .
cat ~/.local/share/monyacode/logs/MonyaCode.log
# copy the output
```

Or, if you have the MonyaCode cli setup, you can do

```sh
MONYACODE_LOG=wgpu=info /path/to/monyacode/cli --foreground .
# copy the output
```

### Forcing X11 scale factor

On X11 systems, MonyaCode automatically detects the appropriate scale factor for
high-DPI displays. The scale factor is determined using the following priority
order:

1. `GPUI_X11_SCALE_FACTOR` environment variable (if set)
2. `Xft.dpi` from X resources database (xrdb)
3. Automatic detection via RandR based on monitor resolution and physical size

If you want to customize the scale factor beyond what MonyaCode detects
automatically, you have several options:

#### Check your current scale factor

You can verify if you have `Xft.dpi` set:

```sh
xrdb -query | grep Xft.dpi
```

If this command returns no output, MonyaCode is using RandR (X11's monitor management
extension) to automatically calculate the scale factor based on your monitor's
reported resolution and physical dimensions.

#### Option 1: Set Xft.dpi (X Resources Database)

`Xft.dpi` is a standard X11 setting that many applications use for consistent
font and UI scaling. Setting this ensures MonyaCode scales the same way as other X11
applications that respect this setting.

Edit or create the `~/.Xresources` file:

```sh
vim ~/.Xresources
```

Add this line with your desired DPI:

```sh
Xft.dpi: 96
```

Common DPI values:

- `96` for standard 1x scaling
- `144` for 1.5x scaling
- `192` for 2x scaling
- `288` for 3x scaling

Load the configuration:

```sh
xrdb -merge ~/.Xresources
```

Restart MonyaCode for the changes to take effect.

#### Option 2: Use the GPUI_X11_SCALE_FACTOR environment variable

This MonyaCode-specific environment variable directly sets the scale factor,
bypassing all automatic detection.

```sh
GPUI_X11_SCALE_FACTOR=1.5 monyacode
```

You can use decimal values (e.g., `1.25`, `1.5`, `2.0`) or set
`GPUI_X11_SCALE_FACTOR=randr` to force RandR-based detection even when `Xft.dpi`
is set.

To make this permanent, add it to your shell profile or desktop entry.

#### Option 3: Adjust system-wide RandR DPI

This changes the reported DPI for your entire X11 session, affecting how RandR
calculates scaling for all applications that use it.

Add this to your `.xprofile` or `.xinitrc`:

```sh
xrandr --dpi 192
```

Replace `192` with your desired DPI value. This affects the system globally and
will be used by MonyaCode's automatic RandR detection when `Xft.dpi` is not set.

### Font rendering parameters

On Linux, the `MONYACODE_FONTS_GAMMA` and `MONYACODE_FONTS_GRAYSCALE_ENHANCED_CONTRAST`
environment variables are read for the values to use for font rendering.

`MONYACODE_FONTS_GAMMA` corresponds to
[getgamma](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwriterenderingparams-getgamma)
values. Allowed range [1.0, 2.2], other values are clipped. Default: 1.8

`MONYACODE_FONTS_GRAYSCALE_ENHANCED_CONTRAST` corresponds to
[getgrayscaleenhancedcontrast](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_1/nf-dwrite_1-idwriterenderingparams1-getgrayscaleenhancedcontrast)
values. Allowed range: [0.0, ..), other values are clipped. Default: 1.0
