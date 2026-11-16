// Every periodic timer of esp_timer skips what a leap of the chip's counter made it miss.
//
// esp_timer calls the callback of a periodic timer once for every period the counter went by (esp_timer.c, timer_process_alarm)
// unless the timer was made with skip_unhandled_events. After a leap forward of hours that is millions of calls in a row in
// the task of esp_timer, core 0 is held, the idle task does not feed the task watchdog and the board falls (mDNS, 05.10).
// No periodic timer of this image has anything to catch up with: IDF itself sets the flag on its task watchdog's. The timers
// that come without it - ETSTimer, which the Wi-Fi library arms, the PHY's, mDNS's - cannot be reached one at a time, so
// esp_timer_create is wrapped (the flag is on the final link line, fell_where::emit_link_args) and every timer is made
// with the flag. It means nothing to a timer that fires once.
#include "esp_timer.h"

esp_err_t __real_esp_timer_create(const esp_timer_create_args_t *args, esp_timer_handle_t *out_handle);

esp_err_t __wrap_esp_timer_create(const esp_timer_create_args_t *args, esp_timer_handle_t *out_handle)
{
    if (args == NULL) {
        return __real_esp_timer_create(args, out_handle);
    }
    esp_timer_create_args_t skipping = *args;
    skipping.skip_unhandled_events = true;
    return __real_esp_timer_create(&skipping, out_handle);
}
