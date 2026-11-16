# The ways down to the recovery image - the hold of the board's button and the count of short starts - in C, here
# beside this file: gesture.c, gesture_hold.c, reset_count.c and their headers began as a copy of the gesture module of
# the MicroPython image (frozen into it); from here on they are this tree's own.
set(gesture_dir ${CMAKE_CURRENT_LIST_DIR})
foreach(name gesture.c gesture_hold.c reset_count.c)
    if(NOT EXISTS "${gesture_dir}/${name}")
        message(FATAL_ERROR "ways_down: ${gesture_dir}/${name} is not there")
    endif()
endforeach()

# The button comes from the board's board.yml through tools/board_env.py, as board_block.cmake reads it for MicroPython.
if("$ENV{STRAGA_GESTURE_PIN}" STREQUAL "" OR "$ENV{STRAGA_GESTURE_ACTIVE}" STREQUAL "")
    message(FATAL_ERROR "ways_down: no button for the board - its board.yml needs a gesture field "
                        "(pin: GPIO<n>, level: low|high); tools/build.sh exports it")
endif()

idf_component_register(SRCS "${gesture_dir}/gesture.c" "${gesture_dir}/gesture_hold.c" "${gesture_dir}/reset_count.c"
                       INCLUDE_DIRS "${gesture_dir}"
                       PRIV_REQUIRES esp_driver_gpio app_update esp_partition nvs_flash esp_timer)

# GESTURE_TASK_PRIORITY 6: above the firmware's own tasks - its threads (CONFIG_PTHREAD_TASK_PRIO_DEFAULT 5) and the
# HTTP server (esp-idf-svc, task_priority 5) - so a busy one does not starve the button; below IDF's system tasks
# (tcpip 18, the event loop 20, esp_timer and Wi-Fi above), which it must not delay. MicroPython keeps its own 2.
# GESTURE_TASK_CORE 0: every task of this firmware is born on its core; the button's is the network's side,
# since it polls a pin and writes nvs and keeps no time with the hardware of core 1. MicroPython leaves it unpinned.
target_compile_definitions(${COMPONENT_LIB} PRIVATE
    STRAGA_GESTURE_PIN=$ENV{STRAGA_GESTURE_PIN}
    STRAGA_GESTURE_ACTIVE=$ENV{STRAGA_GESTURE_ACTIVE}
    GESTURE_TASK_PRIORITY=6
    GESTURE_TASK_CORE=0)
