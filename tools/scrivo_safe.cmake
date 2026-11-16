# scrivo-safe for an IDF component of this tree: the one tools/env.sh names in SCRIVO_SAFE, and without it the
# folder beside the tree. A copy of the tree for a branch lies where nothing is beside it, and tools/build.sh sources
# env.sh, which takes SCRIVO_SAFE from the builder.
#
#   include(${CMAKE_CURRENT_LIST_DIR}/../../../tools/scrivo_safe.cmake)
#   scrivo_safe_dir(scrivo_safe)
#   scrivo_needs(<component> "${scrivo_safe}/main/..." ...)

function(scrivo_safe_dir out)
    if(NOT "$ENV{SCRIVO_SAFE}" STREQUAL "")
        set(${out} "$ENV{SCRIVO_SAFE}" PARENT_SCOPE)
    else()
        get_filename_component(tree "${CMAKE_CURRENT_FUNCTION_LIST_DIR}/.." ABSOLUTE)
        get_filename_component(guessed "${tree}/../scrivo-safe" ABSOLUTE)
        set(${out} "${guessed}" PARENT_SCOPE)
    endif()
endfunction()

# Stops the build in words when a file a component takes from scrivo-safe is not there: which component, which file,
# and where scrivo-safe was taken from - rather than a compiler's "no such directory" about a path nobody wrote.
function(scrivo_needs component)
    foreach(path ${ARGN})
        if(NOT EXISTS "${path}")
            if(NOT "$ENV{SCRIVO_SAFE}" STREQUAL "")
                set(from "SCRIVO_SAFE names $ENV{SCRIVO_SAFE} as scrivo-safe")
            else()
                set(from "SCRIVO_SAFE is not set, so scrivo-safe was taken as the folder beside this tree - "
                         "put scrivo-safe next to the tree or set SCRIVO_SAFE to its folder (tools/env.sh)")
            endif()
            message(FATAL_ERROR "${component}: ${path} is not there; ${from}")
        endif()
    endforeach()
endfunction()
