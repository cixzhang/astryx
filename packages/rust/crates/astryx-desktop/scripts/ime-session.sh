#!/bin/sh
# Runs a command in an X session with a real input method: Xvfb, a D-Bus
# session, ibus over XIM with its Japanese engine (anthy) in hiragana
# mode. For the window_ime test:
#   scripts/ime-session.sh cargo test -p astryx-desktop --test window_ime
# Needs Xvfb, dbus-launch, ibus and ibus-anthy.
set -eu
DISPLAY=:${IME_DISPLAY:-93}
export DISPLAY
Xvfb "$DISPLAY" -screen 0 1280x900x24 >/dev/null 2>&1 &
XVFB=$!
trap 'kill $XVFB 2>/dev/null; [ -n "${DBUS_SESSION_BUS_PID:-}" ] && kill "$DBUS_SESSION_BUS_PID" 2>/dev/null; true' EXIT
sleep 1
eval "$(dbus-launch --sh-syntax)"
export XMODIFIERS=@im=ibus GTK_IM_MODULE=ibus
# Hiragana from the first key, no on/off toggle needed.
gsettings set org.freedesktop.ibus.engine.anthy.common input-mode 0 2>/dev/null || true
gsettings set org.freedesktop.ibus.engine.anthy.common behavior-on-focus-out 0 2>/dev/null || true
# The engine is Python: run it with the system's interpreter even when
# another python3 comes first on PATH.
PATH=/usr/bin:/bin:$PATH ibus-daemon --xim --daemonize --replace --panel=disable
for _ in 1 2 3 4 5 6 7 8 9 10; do ibus engine anthy 2>/dev/null && break; sleep 0.5; done
ibus engine || true
"$@"
