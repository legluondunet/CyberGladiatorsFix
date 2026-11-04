#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Optional Linux/Wine launcher for the Windows CyberGladiatorsFix patcher.
# With ordinary commands, suppress Wine fixme noise without masking err/warn
# diagnostics or any CyberGladiatorsFix output. Respect user WINEDEBUG settings.
# With --trace, leave Wine diagnostics alone (the Windows launcher configures
# additional +seh,+loaddll categories for the game process).
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
TRACE=0
for arg do
    if [ "$arg" = "--trace" ]; then TRACE=1; break; fi
done
if [ "$TRACE" -eq 0 ] && [ "${WINEDEBUG+x}" != x ]; then
    WINEDEBUG=fixme-all
    export WINEDEBUG
fi
exec "${CYBER_WINE:-wine}" "$SCRIPT_DIR/CyberGladiatorsFix.exe" "$@"
