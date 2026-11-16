#!/bin/zsh
# Every device of the workspace builds for the chip of its board, and its image says so:
#
#   tests/build_devices.sh
#
# For each device: tools/build.sh, then esptool image_info on the image - the chip, the version from the
# device's version.txt and the project name. Prints PASS or FAIL per device; exit 0 when all pass.
cd ${0:A:h}/..
failed=0
for dir in devices/*/; do
  device=${dir:t}
  want_version=$(cat $dir/version.txt)
  if ! out=$(tools/build.sh $device 2>&1); then
    echo "FAIL $device: build failed"; echo "$out" | grep -E "^error|board_env|panicked" | head -5
    failed=1; continue
  fi
  eval $(uv run --no-project --with pyyaml python tools/board_env.py $device)
  info=$( . tools/env.sh >/dev/null 2>&1; esptool.py image_info --version 2 build/$device/$device.bin 2>&1)
  chip=$(echo $info | sed -n 's/^Detected image type: //p')
  version=$(echo $info | sed -n 's/^App version: //p')
  project=$(echo $info | sed -n 's/^Project name: //p')
  case $MCU in                       # the names esptool prints
    esp32s3) want_chip=ESP32-S3 ;;
    esp32) want_chip=ESP32 ;;
    *) want_chip="(no esptool name for $MCU in this test)" ;;
  esac
  if [[ $chip == $want_chip && $version == $want_version && $project == ${SCRIVO_MAIN_PROJECT:-micropython} ]]; then
    echo "PASS $device: $chip, $version, $(echo $out | tail -1 | cut -d: -f2 | cut -d, -f1)"
  else
    echo "FAIL $device: image says $chip / $version / $project, want $want_chip / $want_version / ${SCRIVO_MAIN_PROJECT:-micropython}"
    failed=1
  fi
done
exit $failed
