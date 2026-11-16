# Sourced, not run: `. tools/env.sh`. Activates the SDK this image is built with.
#
# IDF 5.5.2, the one the recovery image in factory is built with, so both images agree on the partition table
# offset, the app description and rollback. Which checkout it is, the builder's environment says: IDF_PATH (the
# esp-idf folder, export.sh in it) and IDF_TOOLS_PATH (the folder install.sh put the tools in), as for any IDF
# build. .cargo/config.toml says fromenv: esp-idf-sys takes this activated IDF and downloads none of its own.
#
# From espup only libclang, for the bindings. Its PATH line would put its own GCC (15.2) ahead of the
# one IDF 5.5.2 installs (14.2); IDF's export stays in charge of the C compiler.
#
# scrivo-safe: the recovery image whose board folders and sources this tree builds with. The folder beside this tree,
# and SCRIVO_SAFE when set before sourcing - a copy of the tree for a branch has no scrivo-safe beside it. Named here,
# once, for everything the build reaches outside this tree - tools/board_env.py and tools/scrivo_safe.cmake.
IDF_WANTED=v5.5.2
export SCRIVO_SAFE=${SCRIVO_SAFE:-${${(%):-%x}:A:h:h:h}/scrivo-safe}
if [[ -z $IDF_PATH || -z $IDF_TOOLS_PATH || ! -f $IDF_PATH/export.sh ]]; then
  echo "tools/env.sh: no IDF $IDF_WANTED to activate - set IDF_PATH (the esp-idf folder, export.sh in it) and IDF_TOOLS_PATH (where install.sh put the tools) before sourcing; now IDF_PATH='$IDF_PATH' IDF_TOOLS_PATH='$IDF_TOOLS_PATH'" >&2
  return 1
fi
if [[ -f $IDF_PATH/version.txt && $(<$IDF_PATH/version.txt) != $IDF_WANTED ]]; then
  echo "tools/env.sh: IDF_PATH $IDF_PATH is $(<$IDF_PATH/version.txt), the images are built with $IDF_WANTED" >&2
  return 1
fi
export IDF_TOOLS_PATH IDF_PATH
. "$IDF_PATH/export.sh" >/dev/null
if [[ -f $HOME/export-esp.sh ]]; then
  export LIBCLANG_PATH="$(sed -n 's/^export LIBCLANG_PATH="\(.*\)"$/\1/p' "$HOME/export-esp.sh")"
fi
