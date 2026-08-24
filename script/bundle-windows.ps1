[CmdletBinding()]
Param(
    [Parameter()][Alias('i')][switch]$Install,
    [Parameter()][Alias('h')][switch]$Help,
    [Parameter()][Alias('a')][string]$Architecture,
    [Parameter()][string]$Name
)

. "$PSScriptRoot/lib/workspace.ps1"

# https://stackoverflow.com/questions/57949031/powershell-script-stops-if-program-fails-like-bash-set-o-errexit
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$buildSuccess = $false

$OSArchitecture = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64" { "x86_64" }
    "Arm64" { "aarch64" }
    default { throw "Unsupported architecture" }
}

$Architecture = if ($Architecture) {
    $Architecture
} else {
    $OSArchitecture
}

$CargoOutDir = "./target/$Architecture-pc-windows-msvc/release"

function Get-VSArch {
    param(
        [string]$Arch
    )

    switch ($Arch) {
        "x86_64" { "amd64" }
        "aarch64" { "arm64" }
    }
}

$VSDevPath = Join-Path -Path ((& 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe' -latest -format json | ConvertFrom-Json).installationPath) -ChildPath "\Common7\Tools\Launch-VsDevShell.ps1"

Push-Location
& $VSDevPath -Arch (Get-VSArch -Arch $Architecture) -HostArch (Get-VSArch -Arch $OSArchitecture)
Pop-Location

$target = "$Architecture-pc-windows-msvc"

if ($Help) {
    Write-Output "Usage: test.ps1 [-Install] [-Help]"
    Write-Output "Build the installer for Windows.\n"
    Write-Output "Options:"
    Write-Output "  -Architecture, -a Which architecture to build (x86_64 or aarch64)"
    Write-Output "  -Install, -i      Run the installer after building."
    Write-Output "  -Help, -h         Show this help message."
    exit 0
}

Push-Location -Path crates/monyacode
$channel = Get-Content "RELEASE_CHANNEL"
$env:MONYACODE_RELEASE_CHANNEL = $channel
$env:RELEASE_CHANNEL = $channel
Pop-Location

function CheckEnvironmentVariables {
    $requiredVars = @(
        'MONYACODE_WORKSPACE', 'RELEASE_VERSION', 'MONYACODE_RELEASE_CHANNEL'
    )

    foreach ($var in $requiredVars) {
        if (-not (Test-Path "env:$var")) {
            Write-Error "$var is not set"
            exit 1
        }
    }
}

function PrepareForBundle {
    if (Test-Path "$innoDir") {
        Remove-Item -Path "$innoDir" -Recurse -Force
    }
    New-Item -Path "$innoDir" -ItemType Directory -Force
    Copy-Item -Path "$env:MONYACODE_WORKSPACE\crates\monyacode\resources\windows\*" -Destination "$innoDir" -Recurse -Force
    New-Item -Path "$innoDir\bin" -ItemType Directory -Force
    New-Item -Path "$innoDir\tools" -ItemType Directory -Force

    rustup target add $target
}

function GenerateLicenses {
    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    . $PSScriptRoot/generate-licenses.ps1
    $ErrorActionPreference = $oldErrorActionPreference
}

function BuildMonyaCodeAndItsFriends {
    Write-Output "Building MonyaCode and its friends, for channel: $channel"
    # Build monyacode.exe, cli.exe
    cargo build --release --package monyacode --package cli --target $target
    Copy-Item -Path ".\$CargoOutDir\monyacode.exe" -Destination "$innoDir\MonyaCode.exe" -Force
    Copy-Item -Path ".\$CargoOutDir\cli.exe" -Destination "$innoDir\cli.exe" -Force
}

function ZipMonyaCodeAndItsFriendsDebug {
    $items = @(
        ".\$CargoOutDir\monyacode.pdb",
        ".\$CargoOutDir\cli.pdb"
    )

    Compress-Archive -Path $items -DestinationPath ".\$CargoOutDir\monyacode-$env:RELEASE_VERSION-$env:MONYACODE_RELEASE_CHANNEL.dbg.zip" -Force
}

function CollectFiles {
    Move-Item -Path "$innoDir\cli.exe" -Destination "$innoDir\bin\monyacode.exe" -Force
    Move-Item -Path "$innoDir\monyacode.sh" -Destination "$innoDir\bin\monyacode" -Force
}

function BuildInstaller {
    $issFilePath = "$innoDir\monyacode.iss"
    switch ($channel) {
        "stable" {
            $appId = "{{E62BA84E-40DF-471F-97EF-B85924F488FB}"
            $appIconName = "app-icon"
            $appName = "MonyaCode"
            $appDisplayName = "MonyaCode"
            $appSetupName = "MonyaCode-$Architecture"
            # The mutex name here should match the mutex name in crates\monyacode\src\monyacode\windows_only_instance.rs
            $appMutex = "MonyaCode-Stable-Instance-Mutex"
            $appExeName = "MonyaCode"
            $regValueName = "MonyaCode"
            $appUserId = "MonyaCode.MonyaCode"
            $appShellNameShort = "M&onyaCode"
        }
        "dev" {
            $appId = "{{4FEF353A-EA46-468C-95DD-2B343A71416F}"
            $appIconName = "app-icon-dev"
            $appName = "MonyaCode Dev"
            $appDisplayName = "MonyaCode Dev"
            $appSetupName = "MonyaCode-$Architecture"
            # The mutex name here should match the mutex name in crates\monyacode\src\monyacode\windows_only_instance.rs
            $appMutex = "MonyaCode-Dev-Instance-Mutex"
            $appExeName = "MonyaCode"
            $regValueName = "MonyaCodeDev"
            $appUserId = "MonyaCode.MonyaCode.Dev"
            $appShellNameShort = "M&onyaCode Dev"
        }
        default {
            Write-Error "can't bundle installer for $channel."
            exit 1
        }
    }

    # Windows runner 2022 default has iscc in PATH, https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md
    # Currently, we are using Windows 2022 runner.
    # Windows runner 2025 doesn't have iscc in PATH for now, https://github.com/actions/runner-images/issues/11228
    $innoSetupPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"

    $definitions = @{
        "AppId"          = $appId
        "AppIconName"    = $appIconName
        "OutputDir"      = "$env:MONYACODE_WORKSPACE\target"
        "AppSetupName"   = $appSetupName
        "AppName"        = $appName
        "AppDisplayName" = $appDisplayName
        "RegValueName"   = $regValueName
        "AppMutex"       = $appMutex
        "AppExeName"     = $appExeName
        "ResourcesDir"   = "$innoDir"
        "ShellNameShort" = $appShellNameShort
        "AppUserId"      = $appUserId
        "Version"        = "$env:RELEASE_VERSION"
        "SourceDir"      = "$env:MONYACODE_WORKSPACE"
    }

    $defs = @()
    foreach ($key in $definitions.Keys) {
        $defs += "/d$key=`"$($definitions[$key])`""
    }

    $innoArgs = @($issFilePath) + $defs

    # Execute Inno Setup
    Write-Host "🚀 Running Inno Setup: $innoSetupPath $innoArgs"
    $process = Start-Process -FilePath $innoSetupPath -ArgumentList $innoArgs -NoNewWindow -Wait -PassThru

    if ($process.ExitCode -eq 0) {
        Write-Host "✅ Inno Setup successfully compiled the installer"
        Write-Output "SETUP_PATH=target/$appSetupName.exe" >> $env:GITHUB_ENV
        $script:buildSuccess = $true
    }
    else {
        Write-Host "❌ Inno Setup failed: $($process.ExitCode)"
        $script:buildSuccess = $false
    }
}

ParseMonyaCodeWorkspace
$innoDir = "$env:MONYACODE_WORKSPACE\inno\$Architecture"
$debugArchive = "$CargoOutDir\monyacode-$env:RELEASE_VERSION-$env:MONYACODE_RELEASE_CHANNEL.dbg.zip"
$debugStoreKey = "$env:MONYACODE_RELEASE_CHANNEL/monyacode-$env:RELEASE_VERSION-$env:MONYACODE_RELEASE_CHANNEL.dbg.zip"

CheckEnvironmentVariables
PrepareForBundle
GenerateLicenses
BuildMonyaCodeAndItsFriends
ZipMonyaCodeAndItsFriendsDebug
CollectFiles
BuildInstaller

if ($buildSuccess) {
    Write-Output "Build successful"
    if ($Install) {
        Write-Output "Installing MonyaCode..."
        Start-Process -FilePath "$env:MONYACODE_WORKSPACE/target/MonyaCodeUserSetup-x64-$env:RELEASE_VERSION.exe"
    }
    exit 0
}
else {
    Write-Output "Build failed"
    exit 1
}
