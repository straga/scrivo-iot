// The note the board leaves itself about the fall it did not survive: where it faulted, what the chip called
// it, and the addresses up the stack of the task that fell.
//
// WITHOUT THIS, A BOARD WITH ITS WIRE OFF KNOWS THAT IT PANICKED AND NOT WHERE. The chip's panic handler
// prints the place and the backtrace into the console it has - the USB wire - and then restarts, which takes
// the console's buffer with it. The note is those same numbers put where a restart does not reach: RTC memory,
// which keeps its contents across every reset and loses them only when the power goes.
//
// It costs no partition and no rewrite of flash, so it works from the next fall on.
#pragma once

#include <stdint.h>

// How far up the stack the note keeps addresses. A Rust panic spends twelve frames on its own way to abort()
// before the first frame of ours, so a note that stopped at twelve would name nothing but the machinery.
#define PANIC_NOTE_TRACE  24
// The chip's own word for the exception - "StoreProhibited" and the like - or, for a deliberate abort, the
// details it was given: "abort() was called at PC 0x4037d2c5 on core 1".
#define PANIC_NOTE_REASON 64
// What our own code said before it gave up: a Rust panic's file, line and message.
#define PANIC_NOTE_SAID   96

typedef struct {
    uint32_t seq;                       // the board's count of starts when it fell; 0 when nothing is recorded
    uint32_t exception;                 // panic_exception_t of ESP-IDF
    uint32_t core;                      // the core that took the exception
    uint32_t addr;                      // the instruction that faulted
    uint32_t depth;                     // how many of trace are filled
    uint32_t trace[PANIC_NOTE_TRACE];   // return addresses, the faulting instruction first
    char reason[PANIC_NOTE_REASON];
    char said[PANIC_NOTE_SAID];
    uint32_t image;                     // which image fell: the first four bytes of its ELF hash
    // THE OTHER CORE, AND WHERE IT WAS: a watchdog's answer is half on the core the fall did not happen on.
    // PANIC_NOTE_NO_CORE on a chip with one core; other_depth 0 when that core
    // never reached the panic handler and left no frame to walk.
    uint32_t other_core;
    uint32_t other_depth;
    uint32_t other_trace[PANIC_NOTE_TRACE];
    uint32_t excvaddr;                  // the address a faulting load or store was made at; 0 when none
    // Where the chip's word for the fall lies when it lies in flash: the fall does not read flash, and the next start
    // copies the text from here into reason, when the image is the one that fell. 0 when reason was copied at the fall.
    uint32_t reason_at;
} panic_note_t;

// other_core of a chip that has no other core.
#define PANIC_NOTE_NO_CORE 0xFFFFFFFFu

// Counts this start and hands back the note a fall left, or NULL when there is none. Called once, from the
// firmware's first lines: the count it keeps is what tells a fresh note from one several starts old.
//
// A note that does not check out - RTC memory comes up as whatever the power left in it - is thrown away
// rather than read, and this start becomes the first.
const panic_note_t *panic_note_rise(void);

// The board's count of starts since its power came on, this one included.
uint32_t panic_note_starts(void);

// Which image is running: the first four bytes of its ELF hash, to be held against the note's own.
//
// RTC MEMORY OUTLIVES A FIRMWARE UPDATE, and an address from the build before it points at whatever the new
// build put there - a real function, in the wrong code, reading exactly as confidently as a true one. This is
// what tells the two apart.
uint32_t panic_note_image(void);

// What our own code said before it gave up, for the note. A Rust panic knows its file and line, which no
// address does, and the chip never sees it: the panic hook calls this on its way to abort().
void panic_note_said(const char *text);
