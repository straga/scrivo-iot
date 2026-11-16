#pragma once

// Starts the ways down to the recovery image: counts this start among short starts in a row (three switch boot to
// factory), and watches the board's button for a hold of 10 s (released, it switches boot to factory). Call once at
// start, with nvs initialised and before the firmware's own tasks. The pin comes from the build: STRAGA_GESTURE_PIN
// and STRAGA_GESTURE_ACTIVE, taken from the board's board.yml; GESTURE_TASK_PRIORITY may be set above the firmware's
// busiest tasks.
void ways_down_start(void);
