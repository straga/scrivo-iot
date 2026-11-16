# The tick of FreeRTOS on the systimer, ours in place of IDF's: it gets in front of IDF's setup by wrapping
# vPortSetupTimer, and the flag that does so is on the final link line, emitted by each device's build.rs
# (fell_where::emit_link_args).
# timers.c: esp_timer_create wrapped the same way, so that every periodic timer skips what a leap made it miss.
idf_component_register(SRCS "systick.c" "timers.c"
                       INCLUDE_DIRS "."
                       PRIV_REQUIRES freertos esp_hw_support esp_system hal soc esp_driver_gptimer esp_timer)
