#!/usr/bin/env bash
# Every row that runs on the computer, in one command: `tools/test_host.sh`.
#
# Two runs and not one, because two drivers of embassy-time do not link into one build: the crates below use the std
# driver, `between_leaps` the board's own (`between::steady_time`) over a chip counter that its rows move by hand. `--workspace` does not
# work on the host either: the devices and `firmware` pull esp-idf-sys. A crate that builds on the host is added to the
# first run; `app_desc`, `firmware`, `round`, `vfs`, `ways_down` do not build there.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo +stable test \
    -p analog -p automaton -p automaton_card -p between -p bl0939 -p board_name -p burner_api -p bench_dualr3_api -p bench_s3_api -p burner_map -p curve -p demand \
    -p burner_settings -p dictionary -p dimmer -p draught -p energy -p events -p fell_where -p flame -p flash -p journal -p loudness \
    -p max6675 -p modbus -p module -p page -p pellets -p plan -p regulator -p relay -p rescue -p reset_why -p roam \
    -p schedule -p settings -p shell -p soot -p switch -p tally -p thermostat -p version -p web
cargo +stable test -p between_leaps
# the plan of a board with no external RAM is the other half of crate `plan`: the build of such a board is told STRAGA_PSRAM=0
STRAGA_PSRAM=0 cargo +stable test -p plan
# The public tree carries nothing of ours: the export of this revision, judged by tools/kitchen_check.py, has no findings.
# export.sh and this row stay home, so the public tree skips it.
if [ -f tools/test_public_tree.py ]; then
    uv run --no-project --with pytest pytest -q -p no:cacheprovider tools/test_public_tree.py
fi
