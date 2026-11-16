// Taken from ESP-IDF v5.5.2, components/freertos/port_systick.c (vSystimerSetup, SysTickIsrHandler; Apache-2.0,
// Espressif Systems): the setup as it is, the handler with the leap taken for a leap. Put in front of IDF's by wrapping
// vPortSetupTimer - the flag is on the final link line (fell_where::emit_link_args); IDF's own setup and handler are
// then referenced by nothing, and the link drops them.
#include "systick.h"

#include "sdkconfig.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#if !CONFIG_FREERTOS_SYSTICK_USES_SYSTIMER

// A chip whose tick is not the systimer's (the ESP32 of bench_dualr3): IDF's own setup, and no leap to count.
void __real_vPortSetupTimer(void);

void __wrap_vPortSetupTimer(void)
{
    __real_vPortSetupTimer();
}

systick_leaps_t systick_leaps(void)
{
    systick_leaps_t none = {0, 0, 0, 0, 0, 0};
    return none;
}

int systick_watch(void)
{
    return 0;
}

#else

#include "esp_attr.h"
#include "esp_err.h"
#include "esp_intr_alloc.h"
#include "esp_private/periph_ctrl.h"
#include "esp_private/systimer.h"
#include "hal/systimer_hal.h"
#include "hal/systimer_ll.h"
#include "soc/periph_defs.h"
#include "driver/gptimer.h"

BaseType_t xPortSysTickHandler(void);

// A tick late by more than this - 100 ms at 1000 Hz - is a leap of the counter, not an interrupt served late: the tick's
// interrupt runs from IRAM and is held only by critical sections, a few milliseconds at most. A late tick taken for a
// leap costs the ticks it missed once; a leap taken for a late tick costs seconds of both cores in the interrupt.
#define LEAP_TICKS 100

static systimer_hal_context_t s_hal;
static uint32_t s_handled[configNUM_CORES];
static uint32_t s_leaps[configNUM_CORES];
static uint32_t s_back[configNUM_CORES];
static uint32_t s_forward[configNUM_CORES];
// Ticks served on each core, and the times the watcher found a core without one and set its alarm again.
static volatile uint32_t s_served[configNUM_CORES];
static uint32_t s_rearms;
// The last leap on either core: its size in ticks and whether it went back. Two plain words, written in the interrupt and read by the board once a second; a read between the two
// can pair the size of one leap with the way of another, and the line it makes says so as little as it can (a leap at that very second).
static volatile uint32_t s_last_ticks;
static volatile uint32_t s_last_back;

// Sets the alarm ALARM_ID to fire a period from the counter as it is now. In period mode the alarm keeps its target
// from the last one - a leap of the counter back leaves it seconds ahead and no tick comes (the interrupt watchdog
// resets the board, the burner 26.09 15:39-15:59), a leap forward leaves it behind and it fires without end. Loading
// the period again (comp_load) counts the target from now: what systimer_hal_set_alarm_period does, register by
// register, from IRAM.
//
// ONE LOCK FOR ALL WHO CALL IT: the tick of core 0, the tick of core 1 and the watcher each may, and enabling an alarm is
// a read-modify-write of the timer's one register conf - two at once lose one's bit, and that alarm stays off. The lock
// is esp_timer's own over the systimer (s_time_update_lock, esp_timer_impl_common.c), the one IDF takes at run time to
// touch it, so ours and IDF's writes to the timer take turns too. A spinlock taken with interrupts off: safe from a
// task and from an interrupt, on either core.
extern portMUX_TYPE s_time_update_lock;

static void IRAM_ATTR rearm(uint32_t alarm_id)
{
    portENTER_CRITICAL_SAFE(&s_time_update_lock);
    uint32_t period = systimer_ll_get_alarm_period(s_hal.dev, alarm_id);
    systimer_ll_enable_alarm(s_hal.dev, alarm_id, false);
    systimer_ll_set_alarm_period(s_hal.dev, alarm_id, period);
    systimer_ll_apply_alarm_value(s_hal.dev, alarm_id);
    systimer_ll_enable_alarm(s_hal.dev, alarm_id, true);
    portEXIT_CRITICAL_SAFE(&s_time_update_lock);
}

static void IRAM_ATTR tick(void *arg)
{
    uint32_t cpuid = xPortGetCoreID();
    systimer_hal_context_t *systimer_hal = (systimer_hal_context_t *)arg;
    uint32_t alarm_id = SYSTIMER_ALARM_OS_TICK_CORE0 + cpuid;
    do {
        systimer_ll_clear_alarm_int(systimer_hal->dev, alarm_id);
        uint32_t now = systimer_hal_get_counter_value(systimer_hal, SYSTIMER_COUNTER_OS_TICK) / systimer_ll_get_alarm_period(systimer_hal->dev, alarm_id);
        uint32_t diff = now - s_handled[cpuid];
        if (s_handled[cpuid] == 0) {
            s_handled[cpuid] = now;
            diff = 1;
        } else if (diff > LEAP_TICKS) {
            // back reads as a difference past half the range: the counter below the last tick handled
            s_leaps[cpuid]++;
            if (diff > UINT32_MAX / 2) {
                uint32_t back = s_handled[cpuid] - now;
                s_last_ticks = back;
                s_last_back = 1;
                s_back[cpuid] = back > s_back[cpuid] ? back : s_back[cpuid];
            } else {
                s_last_ticks = diff;
                s_last_back = 0;
                s_forward[cpuid] = diff > s_forward[cpuid] ? diff : s_forward[cpuid];
            }
            s_handled[cpuid] = now;
            diff = 1;
            rearm(alarm_id);
        } else {
            s_handled[cpuid] += diff;
        }
        s_served[cpuid]++;
        while (diff > 0) {
            xPortSysTickHandler();
            diff--;
        }
    } while (systimer_ll_is_alarm_int_fired(systimer_hal->dev, alarm_id));
}

// IDF's vSystimerSetup, registering tick() instead of SysTickIsrHandler.
void __wrap_vPortSetupTimer(void)
{
    unsigned cpuid = xPortGetCoreID();
#ifdef CONFIG_FREERTOS_CORETIMER_SYSTIMER_LVL3
    const unsigned level = ESP_INTR_FLAG_LEVEL3;
#else
    const unsigned level = ESP_INTR_FLAG_LEVEL1;
#endif
    systimer_hal_context_t *const hal = &s_hal;
    ESP_ERROR_CHECK(esp_intr_alloc(ETS_SYSTIMER_TARGET0_INTR_SOURCE + cpuid, ESP_INTR_FLAG_IRAM | level, tick, hal, NULL));

    if (cpuid == 0) {
        PERIPH_RCC_ACQUIRE_ATOMIC(PERIPH_SYSTIMER_MODULE, ref_count) {
            if (ref_count == 0) {
                systimer_ll_enable_bus_clock(true);
                systimer_ll_reset_register();
            }
        }
        systimer_hal_init(hal);
        systimer_hal_tick_rate_ops_t ops = {
            .ticks_to_us = systimer_ticks_to_us,
            .us_to_ticks = systimer_us_to_ticks,
        };
        systimer_hal_set_tick_rate_ops(hal, &ops);
        systimer_ll_set_counter_value(hal->dev, SYSTIMER_COUNTER_OS_TICK, 0);
        systimer_ll_apply_counter_value(hal->dev, SYSTIMER_COUNTER_OS_TICK);

        for (cpuid = 0; cpuid < SOC_CPU_CORES_NUM; cpuid++) {
            systimer_hal_counter_can_stall_by_cpu(hal, SYSTIMER_COUNTER_OS_TICK, cpuid, false);
            uint32_t alarm_id = SYSTIMER_ALARM_OS_TICK_CORE0 + cpuid;
            systimer_hal_select_alarm_mode(hal, alarm_id, SYSTIMER_ALARM_MODE_ONESHOT);
        }

        for (cpuid = 0; cpuid < configNUM_CORES; ++cpuid) {
            uint32_t alarm_id = SYSTIMER_ALARM_OS_TICK_CORE0 + cpuid;
            systimer_hal_connect_alarm_counter(hal, alarm_id, SYSTIMER_COUNTER_OS_TICK);
            systimer_hal_set_alarm_period(hal, alarm_id, 1000000UL / CONFIG_FREERTOS_HZ);
            systimer_hal_select_alarm_mode(hal, alarm_id, SYSTIMER_ALARM_MODE_PERIOD);
            systimer_hal_counter_can_stall_by_cpu(hal, SYSTIMER_COUNTER_OS_TICK, cpuid, true);
            if (cpuid == 0) {
                systimer_hal_enable_alarm_int(hal, alarm_id);
                systimer_hal_enable_counter(hal, SYSTIMER_COUNTER_OS_TICK);
#ifndef CONFIG_FREERTOS_UNICORE
                systimer_hal_counter_value_advance(hal, SYSTIMER_COUNTER_OS_TICK, 1000000UL / CONFIG_FREERTOS_HZ / 2);
#endif
            }
        }
    } else {
        uint32_t alarm_id = SYSTIMER_ALARM_OS_TICK_CORE0 + cpuid;
        systimer_hal_enable_alarm_int(hal, alarm_id);
    }
}

systick_leaps_t systick_leaps(void)
{
    systick_leaps_t leaps = {0, 0, 0, 0, 0, 0};
    for (int cpuid = 0; cpuid < configNUM_CORES; cpuid++) {
        leaps.count += s_leaps[cpuid];
        leaps.largest_back = s_back[cpuid] > leaps.largest_back ? s_back[cpuid] : leaps.largest_back;
        leaps.largest_forward = s_forward[cpuid] > leaps.largest_forward ? s_forward[cpuid] : leaps.largest_forward;
    }
    leaps.rearms = s_rearms;
    leaps.last_ticks = s_last_ticks;
    leaps.last_was_back = s_last_back;
    return leaps;
}

// ---- the watcher: a clock that is not the systimer's ----

// How often the watcher looks, and how many looks without a tick make a core's alarm set again: 20 ms without a tick
// at 1000 Hz is no late tick but a silent alarm, and the interrupt watchdog (CONFIG_ESP_INT_WDT_TIMEOUT_MS) is far off.
#define WATCH_US 10000
#define SILENT_LOOKS 2

static uint32_t s_seen[configNUM_CORES];
static uint32_t s_silent[configNUM_CORES];

// A leap of the counter back leaves the tick's alarm seconds ahead, and no tick comes to notice it: the watcher does,
// on a timer of its own (a timer group's, not the systimer's), and sets the silent core's alarm from now. The tick
// that follows sees the counter below the last tick handled and takes it for a leap.
static bool IRAM_ATTR watch(gptimer_handle_t timer, const gptimer_alarm_event_data_t *event, void *ctx)
{
    for (uint32_t cpuid = 0; cpuid < configNUM_CORES; cpuid++) {
        uint32_t served = s_served[cpuid];
        if (served != s_seen[cpuid]) {
            s_seen[cpuid] = served;
            s_silent[cpuid] = 0;
        } else if (++s_silent[cpuid] >= SILENT_LOOKS) {
            rearm(SYSTIMER_ALARM_OS_TICK_CORE0 + cpuid);
            s_rearms++;
            s_silent[cpuid] = 0;
        }
    }
    return false;
}

int systick_watch(void)
{
    gptimer_handle_t timer = NULL;
    gptimer_config_t config = {
        .clk_src = GPTIMER_CLK_SRC_DEFAULT,
        .direction = GPTIMER_COUNT_UP,
        .resolution_hz = 1000000,
    };
    esp_err_t err = gptimer_new_timer(&config, &timer);
    gptimer_event_callbacks_t callbacks = {.on_alarm = watch};
    if (err == ESP_OK) {
        err = gptimer_register_event_callbacks(timer, &callbacks, NULL);
    }
    gptimer_alarm_config_t alarm = {.alarm_count = WATCH_US, .reload_count = 0, .flags.auto_reload_on_alarm = true};
    if (err == ESP_OK) {
        err = gptimer_set_alarm_action(timer, &alarm);
    }
    if (err == ESP_OK) {
        err = gptimer_enable(timer);
    }
    if (err == ESP_OK) {
        err = gptimer_start(timer);
    }
    return err;
}

#endif /* CONFIG_FREERTOS_SYSTICK_USES_SYSTIMER */
