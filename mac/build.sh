#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

swift build -c release
BIN="$(swift build -c release --show-bin-path)/ClaudeTray"

APP="build/ClaudeTray.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/ClaudeTray"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>ClaudeTray</string>
    <key>CFBundleDisplayName</key><string>ClaudeTray</string>
    <key>CFBundleIdentifier</key><string>local.claudetray</string>
    <key>CFBundleExecutable</key><string>ClaudeTray</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>LSMinimumSystemVersion</key><string>14.4</string>
    <key>LSUIElement</key><true/>
    <key>NSHighResolutionCapable</key><true/>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
</dict>
</plist>
PLIST

cp -R ../locales/*.lproj "$APP/Contents/Resources/"

codesign --force --sign - --timestamp=none "$APP"
echo "Built $APP"

if [ "${1:-}" = "--install" ]; then
    pkill -x ClaudeTray 2>/dev/null || true
    rm -rf /Applications/ClaudeTray.app
    cp -R "$APP" /Applications/
    open /Applications/ClaudeTray.app
    echo "Installed to /Applications"

    mkdir -p "$HOME/.local/bin"
    cat > "$HOME/.local/bin/claudetray" <<'SH'
#!/bin/sh
exec /Applications/ClaudeTray.app/Contents/MacOS/ClaudeTray "${@:-list}"
SH
    chmod +x "$HOME/.local/bin/claudetray"
    echo "Installed ~/.local/bin/claudetray"
fi
