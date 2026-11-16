#!/bin/zsh
# Builds a device's image, the one the recovery image's page takes for ota_0: build/<device>/<device>.bin.
#
#   tools/build.sh <device> [cargo build options, e.g. --features never-confirm]
#   STRAGA_VARIANT=no_psram tools/build.sh <device>     the board's variant (tools/board_env.py): its own target/ and
#                                                       build/<device>/<device>.<variant>.bin, so no build of the plain one is touched
#
# The device's board (tools/board_env.py) gives the target, the MCU, the IDF files and the flash of the image
# header; a device without a board, or a board without its files, stops here with the reason. The ELF from
# cargo becomes an image with the command IDF itself runs for an app, the ELF hash at offset 0xb0 of the app
# description. Refuses an image larger than ota_0 of the board's partition table.
set -e
cd ${0:A:h}/..
DEVICE=$1
if [[ -z $DEVICE ]]; then
  echo "usage: tools/build.sh <device> [cargo options]; devices: $(ls devices | tr '\n' ' ')" >&2
  exit 2
fi
shift
# The SDK first: it names scrivo-safe (SCRIVO_SAFE), where board_env.py finds the board's folder.
. tools/env.sh
board_env=$(uv run --no-project --with pyyaml python tools/board_env.py $DEVICE ${STRAGA_VARIANT:+--variant} $STRAGA_VARIANT)
# IDF is configured once per target dir: a build with other options goes to a folder of its own.
TARGET_DIR=target${STRAGA_VARIANT:+_$STRAGA_VARIANT}
eval $board_env

# A CRATE'S IDF COMPONENT IS REBUILT ONLY WHEN ITS bindings.h CHANGES, and nothing else in the component is
# watched: esp-idf-sys emits rerun-if-changed for that one header. Edit the component's .c and the old library
# keeps being linked - silently, and with a struct layout the Rust side no longer agrees with. Touching the
# header is what puts the component's sources back under the build's eye.
for header in crates/*/*/bindings.h; do
  component=${header:h}
  if [[ -n $(find $component -type f -newer $header -not -name bindings.h 2>/dev/null) ]]; then
    echo "$component changed: touching ${header} so esp-idf-sys rebuilds it"
    touch $header
  fi
done

# NOR IS scrivo-safe WATCHED: esp-idf-sys reruns on the variables it knows, and SCRIVO_SAFE is not one of them. A
# build against another scrivo-safe would link the components configured from the last one - silently, and a
# scrivo-safe that lacks their sources would never be told so. The scrivo-safe of the last build is kept beside it, and
# a different one touches every component's header, as above.
stamp=$TARGET_DIR/$TARGET/scrivo_safe
scrivo_safe=${SCRIVO_SAFE:A}
if [[ ! -f $stamp || $(<$stamp) != $scrivo_safe ]]; then
  echo "scrivo-safe is $scrivo_safe, not the one of the last build: touching the components' headers so esp-idf-sys rebuilds them"
  touch crates/*/*/bindings.h
fi

cargo build --release -p $DEVICE --target $TARGET --target-dir $TARGET_DIR "$@"
mkdir -p ${stamp:h} && print -r -- $scrivo_safe > $stamp

ELF=$TARGET_DIR/$TARGET/release/$DEVICE
OUT=build/$DEVICE
BIN=$OUT/$DEVICE${STRAGA_VARIANT:+.$STRAGA_VARIANT}.bin
# The chip revisions the image allows, as IDF writes them: from the sdkconfig this build ran with.
sdkconfig=$(ls -t $TARGET_DIR/$TARGET/release/build/esp-idf-sys-*/out/sdkconfig | head -1)
rev_min=$(sed -n 's/^CONFIG_ESP_REV_MIN_FULL=//p' $sdkconfig)
rev_max=$(sed -n 's/^CONFIG_ESP_REV_MAX_FULL=//p' $sdkconfig)
mkdir -p $OUT
esptool.py --chip $MCU elf2image --flash_mode $FLASH_MODE --flash_freq $FLASH_FREQ --flash_size $FLASH_SIZE \
  --elf-sha256-offset 0xb0 --min-rev-full $rev_min --max-rev-full $rev_max -o $BIN $ELF >/dev/null

size=$(stat -f %z $BIN)
slot=$(awk -F, '$1 ~ /^ota_0/ { gsub(/ /, "", $5); print $5 }' $PARTITIONS)
case $slot in
  *K) slot=$(( ${slot%K} * 1024 )) ;;
  *M) slot=$(( ${slot%M} * 1024 * 1024 )) ;;
  *) slot=$(( slot )) ;;
esac
echo "$BIN: $size bytes of $slot in ota_0 ($(( size * 100 / slot ))%), $MCU, version $(cat $DEVICE_DIR/version.txt)"
if (( size > slot )); then
  echo "the image does not fit ota_0 of $PARTITIONS" >&2
  exit 1
fi
