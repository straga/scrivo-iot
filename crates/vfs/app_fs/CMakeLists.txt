# The vfs partition as the recovery image opens it: the same app_fs.c and the same LittleFS v2, both from the
# neighbouring scrivo-safe, built with MicroPython's flags - one configuration for both images, so a partition
# written by one reads in the other (scrivo-safe/main/CMakeLists.txt does the same).
include(${CMAKE_CURRENT_LIST_DIR}/../../../tools/scrivo_safe.cmake)
scrivo_safe_dir(scrivo_safe)
set(scrivo_safe_main ${scrivo_safe}/main)
set(littlefs_dir ${scrivo_safe}/third_party/littlefs)
scrivo_needs(vfs "${scrivo_safe_main}/app_fs.c" "${scrivo_safe_main}/app_fs_tree.c" "${littlefs_dir}/lfs2.c")

idf_component_register(SRCS "${scrivo_safe_main}/app_fs.c" "${scrivo_safe_main}/app_fs_tree.c" "${littlefs_dir}/lfs2.c" "${littlefs_dir}/lfs2_util.c"
                       INCLUDE_DIRS "${scrivo_safe_main}" "${littlefs_dir}"
                       PRIV_REQUIRES esp_partition)

target_compile_definitions(${COMPONENT_LIB} PUBLIC
    LFS2_NO_MALLOC LFS2_NO_DEBUG LFS2_NO_WARN LFS2_NO_ERROR LFS2_NO_ASSERT)
# third-party source: its warnings are not ours to turn into errors
set_source_files_properties("${littlefs_dir}/lfs2.c" "${littlefs_dir}/lfs2_util.c"
    PROPERTIES COMPILE_OPTIONS "-Wno-error;-Wno-unused-function")
