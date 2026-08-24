#!/usr/bin/env sh
set -eu

# Uninstalls MonyaCode that was installed using the install.sh script

check_remaining_installations() {
    platform="$(uname -s)"
    if [ "$platform" = "Darwin" ]; then
        # Check for any MonyaCode variants in /Applications
        remaining=$(ls -d /Applications/MonyaCode*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    else
        # Check for any MonyaCode variants in ~/.local
        remaining=$(ls -d "$HOME/.local/monyacode"*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    fi
}

prompt_remove_preferences() {
    printf "Do you want to keep your MonyaCode preferences? [Y/n] "
    read -r response
    case "$response" in
    [nN] | [nN][oO])
        rm -rf "$HOME/.config/monyacode"
        echo "Preferences removed."
        ;;
    *)
        echo "Preferences kept."
        ;;
    esac
}

main() {
    platform="$(uname -s)"
    channel="${MONYACODE_CHANNEL:-stable}"

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    "$platform"

    echo "MonyaCode has been uninstalled"
}

linux() {
    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    # Remove the app directory
    rm -rf "$HOME/.local/monyacode$suffix.app"

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/monyacode"

    # Remove the .desktop file
    rm -f "$HOME/.local/share/applications/monyacode$suffix.desktop"

    # Remove the database directory for this channel
    rm -rf "$HOME/.local/share/monyacode/db/0-$suffix"

    # Remove socket file
    rm -f "$HOME/.local/share/monyacode/monyacode-$suffix.sock"

    # Remove the entire MonyaCode directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/.local/share/monyacode"
        prompt_remove_preferences
    fi

    rm -rf "$HOME"/.monyacode_server
}

macos() {
    app="MonyaCode.app"
    db_suffix="stable"
    app_id="app.liten.MonyaCode"
    case "$channel" in
    dev)
        app="MonyaCode Dev.app"
        db_suffix="dev"
        app_id="app.liten.MonyaCode-Dev"
        ;;
    esac

    # Remove the app bundle
    if [ -d "/Applications/$app" ]; then
        rm -rf "/Applications/$app"
    fi

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/monyacode"

    # Remove the database directory for this channel
    rm -rf "$HOME/Library/Application Support/MonyaCode/db/0-$db_suffix"

    # Remove app-specific files and directories
    rm -rf "$HOME/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments/$app_id.sfl"*
    rm -rf "$HOME/Library/Caches/$app_id"
    rm -rf "$HOME/Library/HTTPStorages/$app_id"
    rm -rf "$HOME/Library/Preferences/$app_id.plist"
    rm -rf "$HOME/Library/Saved Application State/$app_id.savedState"

    # Remove the entire MonyaCode directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/Library/Application Support/MonyaCode"
        rm -rf "$HOME/Library/Logs/MonyaCode"

        prompt_remove_preferences
    fi

    rm -rf "$HOME"/.monyacode_server
}

main "$@"
