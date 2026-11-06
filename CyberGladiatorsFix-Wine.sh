#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Optional Linux/Wine launcher for the Windows CyberGladiatorsFix patcher.
# With ordinary commands, suppress Wine fixme noise without masking err/warn
# diagnostics or any CyberGladiatorsFix output. Respect user WINEDEBUG settings.
# With --trace, leave Wine diagnostics alone (the Windows launcher configures
# additional +seh,+loaddll categories for the game process).
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# The Windows binary cannot query the native Linux distribution/kernel
# through Win32. Pass only the public OS labels, not usernames, home
# directories, hostnames, or arbitrary environment values.
if [ "$(uname -s 2>/dev/null)" = Linux ]; then
    CYBER_HOST_KERNEL=$(uname -r 2>/dev/null || printf unknown)
    CYBER_HOST_OS=Linux
    if [ -r /etc/os-release ]; then
        os_name=$(sed -n 's/^PRETTY_NAME=//p' /etc/os-release | head -n 1)
        os_name=$(printf '%s' "$os_name" | tr -d '"')
        if [ -n "$os_name" ]; then CYBER_HOST_OS=$os_name; fi
    fi
    export CYBER_HOST_OS CYBER_HOST_KERNEL
fi
TRACE=0
for arg do
    if [ "$arg" = "--trace" ]; then TRACE=1; break; fi
done
if [ "$TRACE" -eq 0 ] && [ "${WINEDEBUG+x}" != x ]; then
    WINEDEBUG=fixme-all
    export WINEDEBUG
fi
exec "${CYBER_WINE:-wine}" "$SCRIPT_DIR/CyberGladiatorsFix.exe" "$@"
