// Ways down to the recovery image in factory, in C, for any firmware on IDF. ways_down_start (gesture.h) is called once at
// start, with nvs up and before the firmware's own tasks: MicroPython calls it from the board's MICROPY_BOARD_STARTUP
// (board_startup.c) before the port creates the MicroPython task, so both work whether or not the interpreter ever
// reaches boot.py; a Rust firmware calls it first thing in main (scrivo-iot crates/ways_down). Nothing here knows either.
//
// Short starts in a row. Three starts in a row from power, the reset line or software - each shorter than
// RESET_COUNT_ALIVE_MS of work - switch boot to factory: the way down when the button cannot be reached, a board in a
// closed case. The count is kept in nvs (RTC memory does not survive the power pulled), written once at a counted
// start and once when the firmware has stayed up, and read before anything of the application runs. A watchdog or a
// panic neither counts nor breaks the row: hangs have their own count in the application (hang_fails). The count
// itself is reset_count.c.
//
// Hold gesture. The button held for GESTURE_HOLD_MS and then released switches boot to factory and restarts. It acts on
// the release, not when the time is up: a restart meets the pin already released, and a pin held low for good - a stuck
// button, the CH343 adapter's DTR line - never trips it. Holding the button while the chip resets
// is another thing: GPIO0 is the download-mode strap. The pin is sampled, not taken by interrupt: the GPIO interrupt
// service stays machine.Pin's alone, and a 50 ms sample does not see contact bounce.
//
// Both run in one task above the interpreter's priority, so a spinning interpreter does not starve them.
#include "esp_rom_sys.h"
#include "esp_rom_uart.h"

#include "driver/gpio.h"
#include "esp_ota_ops.h"
#include "esp_partition.h"
#include "esp_system.h"
#include "esp_task.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "nvs.h"

#include "gesture.h"
#include "gesture_hold.h"
#include "reset_count.h"

#define GESTURE_HOLD_MS (10000)
#define GESTURE_SAMPLE_MS (50)
// the task writes nvs once the firmware has stayed up; the stack left after that write is printed
#define GESTURE_TASK_STACK (4096)
// Above the tasks of the firmware it lies in, so a busy firmware does not starve it. The default is one above the
// MicroPython task (ESP_TASK_PRIO_MIN + 1 in ports/esp32/main.c); a firmware with busier tasks sets its own.
#ifndef GESTURE_TASK_PRIORITY
#define GESTURE_TASK_PRIORITY (ESP_TASK_PRIO_MIN + 2)
#endif
// The core the task is born on. MicroPython leaves it to the scheduler; a firmware that gives every task a core sets
// its own - the task polls a pin and writes nvs, and keeps no time with hardware.
#ifndef GESTURE_TASK_CORE
#define GESTURE_TASK_CORE (tskNO_AFFINITY)
#endif

// the owner's numbers, 15.09: three starts in a row, each shorter than ten seconds of work
#define RESET_COUNT_LIMIT (3)
#define RESET_COUNT_ALIVE_MS (10000)
#define RESET_COUNT_NAMESPACE "scrivo"
// not boot_fails, the key of the application's former count: a value it left behind must not count here
#define RESET_COUNT_KEY "reset_count"

// the count in nvs after this start's own write; valid only when the start could read nvs
static int32_t reset_count_stored;
static bool reset_count_known;

// Switches boot to factory and restarts; returns only when the switch failed, after saying so.
static void to_factory(const char *who) {
    const esp_partition_t *factory = esp_partition_find_first(ESP_PARTITION_TYPE_APP,
        ESP_PARTITION_SUBTYPE_APP_FACTORY, NULL);
    esp_err_t err = factory ? esp_ota_set_boot_partition(factory) : ESP_ERR_NOT_FOUND;
    if (err != ESP_OK) {
        esp_rom_printf("%s: boot not switched to factory: %s\n", who, esp_err_to_name(err));
        return;
    }
    // the lines above are lost if the chip restarts while they sit in the UART FIFO
    esp_rom_output_tx_wait_idle(0);
    esp_restart();
}

// Power, the reset line and software count. On this board the reset button and the CH343 adapter's reset line read
// as power-on (ESP_RST_POWERON, measured 15.09). Watchdogs and panics are hangs, and deep sleep, brownout and the rest
// are not a person resetting the board.
static bool reset_counts(esp_reset_reason_t reason) {
    return reason == ESP_RST_POWERON || reason == ESP_RST_EXT || reason == ESP_RST_SW;
}

static const char *reset_reason_name(esp_reset_reason_t reason) {
    switch (reason) {
        case ESP_RST_PANIC:
            return "a panic";
        case ESP_RST_INT_WDT:
        case ESP_RST_TASK_WDT:
        case ESP_RST_WDT:
            return "a watchdog";
        case ESP_RST_DEEPSLEEP:
            return "deep sleep";
        case ESP_RST_BROWNOUT:
        case ESP_RST_PWR_GLITCH:
            return "a power sag";
        default:
            return "another reset";
    }
}

static esp_err_t reset_count_write(int32_t value) {
    nvs_handle_t handle;
    esp_err_t err = nvs_open(RESET_COUNT_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        return err;
    }
    err = nvs_set_i32(handle, RESET_COUNT_KEY, value);
    if (err == ESP_OK) {
        err = nvs_commit(handle);
    }
    nvs_close(handle);
    return err;
}

static void reset_count_start(void) {
    esp_reset_reason_t reason = esp_reset_reason();
    int32_t stored = 0;
    nvs_handle_t handle;
    esp_err_t err = nvs_open(RESET_COUNT_NAMESPACE, NVS_READONLY, &handle);
    if (err == ESP_OK) {
        err = nvs_get_i32(handle, RESET_COUNT_KEY, &stored);
        nvs_close(handle);
    }
    if (err == ESP_ERR_NVS_NOT_FOUND) {
        // no namespace yet, or no key: no starts in a row
        stored = 0;
        err = ESP_OK;
    }
    if (err != ESP_OK) {
        // a broken count must not stop the firmware from starting
        esp_rom_printf("reset count: nvs unavailable (%s) - this start is not counted\n", esp_err_to_name(err));
        return;
    }
    bool counted = reset_counts(reason);
    reset_count_start_t start = reset_count_on_start(stored, counted, RESET_COUNT_LIMIT);
    reset_count_stored = stored;
    reset_count_known = true;
    if (start.write) {
        err = reset_count_write(start.store);
        if (err != ESP_OK) {
            esp_rom_printf("reset count: nvs not written (%s) - this start is not counted\n", esp_err_to_name(err));
            return;
        }
        reset_count_stored = start.store;
    }

    nvs_stats_t stats;
    if (nvs_get_stats(NULL, &stats) == ESP_OK) {
        esp_rom_printf("reset count: nvs entries used %u of %u\n", (unsigned)stats.used_entries, (unsigned)stats.total_entries);
    }
    if (!counted) {
        esp_rom_printf("reset count: a start after %s is not counted; short starts in a row: %d of %d\n",
            reset_reason_name(reason), (int)start.starts, RESET_COUNT_LIMIT);
    } else if (start.to_factory) {
        esp_rom_printf("reset count: %d short starts in a row - switching to the recovery image in factory\n", (int)start.starts);
        to_factory("reset count");
    } else if (start.starts == RESET_COUNT_LIMIT - 1) {
        esp_rom_printf("reset count: %d short starts in a row - one more reset within %d s takes the board to the recovery "
            "image in factory\n", (int)start.starts, RESET_COUNT_ALIVE_MS / 1000);
    } else {
        esp_rom_printf("reset count: short start %d of %d in a row; it clears after %d s of work\n",
            (int)start.starts, RESET_COUNT_LIMIT, RESET_COUNT_ALIVE_MS / 1000);
    }
}

static void reset_count_alive(void) {
    if (!reset_count_known || !reset_count_on_alive(reset_count_stored)) {
        return;
    }
    esp_err_t err = reset_count_write(0);
    if (err != ESP_OK) {
        esp_rom_printf("reset count: not cleared (%s)\n", esp_err_to_name(err));
        return;
    }
    reset_count_stored = 0;
    esp_rom_printf("reset count: up for %d s - short starts in a row cleared; gesture task stack left %u bytes\n",
        RESET_COUNT_ALIVE_MS / 1000, (unsigned)uxTaskGetStackHighWaterMark(NULL));
}

static void gesture_to_factory(uint32_t held_ms) {
    esp_rom_printf("gesture: button held %u s and released - switching to the recovery image in factory\n",
        (unsigned)(held_ms / 1000));
    to_factory("gesture");
}

static void gesture_task(void *arg) {
    (void)arg;
    gesture_hold_t hold;
    gesture_hold_init(&hold, GESTURE_HOLD_MS, STRAGA_GESTURE_ACTIVE);
    bool alive_checked = false;
    for (;;) {
        uint32_t now_ms = (uint32_t)(esp_timer_get_time() / 1000);
        if (!alive_checked && now_ms >= RESET_COUNT_ALIVE_MS) {
            alive_checked = true;
            reset_count_alive();
        }
        uint32_t held_ms = gesture_hold_step(&hold, gpio_get_level(STRAGA_GESTURE_PIN), now_ms);
        if (held_ms) {
            gesture_to_factory(held_ms);
        }
        vTaskDelay(pdMS_TO_TICKS(GESTURE_SAMPLE_MS));
    }
}

void ways_down_start(void) {
    reset_count_start();
    gpio_config_t pin = {
        .pin_bit_mask = 1ULL << STRAGA_GESTURE_PIN,
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = STRAGA_GESTURE_ACTIVE == 0 ? GPIO_PULLUP_ENABLE : GPIO_PULLUP_DISABLE,
        .pull_down_en = STRAGA_GESTURE_ACTIVE == 0 ? GPIO_PULLDOWN_DISABLE : GPIO_PULLDOWN_ENABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    if (gpio_config(&pin) != ESP_OK
        || xTaskCreatePinnedToCore(gesture_task, "gesture", GESTURE_TASK_STACK, NULL, GESTURE_TASK_PRIORITY, NULL,
                                   GESTURE_TASK_CORE) != pdPASS) {
        // a gesture that failed to start must not stop the firmware from starting; the count is cleared only by this
        // task, so without it a count stays until the next start
        esp_rom_printf("gesture: not started\n");
    }
    // a firmware may set the console up after this returns, and what is still in the FIFO is dropped with it
    esp_rom_output_tx_wait_idle(0);
}
