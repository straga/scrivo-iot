# The note a fall leaves in RTC memory. It gets in front of the chip's panic handler by wrapping it, and the
# flag that does so is on the final link line, emitted by each device's build.rs (fell_where::emit_link_args).
idf_component_register(SRCS "panic_note.c"
                       INCLUDE_DIRS "."
                       PRIV_REQUIRES esp_system esp_hw_support esp_app_format)
