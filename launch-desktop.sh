#!/usr/bin/env bash
set -uo pipefail
umask 077
STUDIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STUDIO_STATE="${XDG_STATE_HOME:-$HOME/.local/state}/canvas-studio"
mkdir -p "$STUDIO_STATE"
chmod 700 "$STUDIO_STATE"
STUDIO_LOG="$STUDIO_STATE/desktop.log"
if [ -f "$STUDIO_LOG" ] && [ "$(stat -c %s "$STUDIO_LOG")" -gt 5242880 ]; then
    mv -f "$STUDIO_LOG" "$STUDIO_LOG.previous"
fi
if [ "$#" -gt 0 ] && [[ "$1" != -* ]]; then
    set -- --open "$1"
fi
"$STUDIO_DIR/run.sh" "$@" >>"$STUDIO_LOG" 2>&1
STUDIO_STATUS=$?
chmod 600 "$STUDIO_LOG"
if [ "$STUDIO_STATUS" -ne 0 ]; then
    if command -v zenity >/dev/null 2>&1; then
        zenity --error --title="Canvas Studio could not start" --text="The desktop app stopped. Launch details are saved in $STUDIO_LOG" 2>/dev/null || true
    elif command -v notify-send >/dev/null 2>&1; then
        notify-send "Canvas Studio could not start" "Launch details: $STUDIO_LOG" || true
    fi
fi
exit "$STUDIO_STATUS"
