#include "panic_note.h"

#include <stddef.h>
#include <string.h>

#include "esp_app_desc.h"
#include "esp_attr.h"
#include "esp_cpu_utils.h"
#include "esp_debug_helpers.h"
#include "esp_memory_utils.h"
#include "esp_private/panic_internal.h"
#include "soc/soc_caps.h"
#include "soc/soc_memory_layout.h"
#include "xtensa_context.h"

// Says the block was written by this firmware and not left by whatever ran before. The checksum below is what
// actually decides; this only keeps a zeroed RTC from reading as a note.
#define KEEP_MAGIC 0x50614E6F

typedef struct {
    uint32_t magic;
    uint32_t starts;
    uint32_t image;          // the running image, taken at the start: a fall stamps the note with it
    panic_note_t note;
    uint32_t sum;
} keep_t;

// RTC memory that no startup code clears: it keeps what is in it across every reset and loses it when the
// power goes, which is the behaviour wanted - a note is about a fall, and a fall does not survive the power.
static RTC_NOINIT_ATTR keep_t s_keep;

// The note as it was before this start counted itself, handed to the firmware to read at leisure.
static panic_note_t s_handed;

// EVERYTHING THE NOTE IS WRITTEN BY LIES IN IRAM (IRAM_ATTR here, CONFIG_ESP_PANIC_HANDLER_IRAM for IDF's side), AND
// IT READS NO FLASH: a fall while the flash chip erases or writes finds code and rodata in flash unreadable for the
// whole erase, and a writer there leaves no note - most falls of the burner on 26.09 left none. The text the fall is
// handed - the reason, the details of an abort - is copied only when it lies inside; one in flash is kept by its
// address and read at the next start (keep_reason, panic_note_rise). tools/panic_path_in_iram.py checks the built
// image.

static IRAM_ATTR uint32_t sum_of(const keep_t *keep)
{
    // FNV-1a over everything but the checksum itself. RTC memory that comes up as noise passes the magic
    // once in four billion; it does not also pass this.
    const uint8_t *bytes = (const uint8_t *)keep;
    uint32_t hash = 2166136261u;
    for (size_t i = 0; i < offsetof(keep_t, sum); i++) {
        hash = (hash ^ bytes[i]) * 16777619u;
    }
    return hash;
}

static IRAM_ATTR void reseal(void)
{
    s_keep.magic = KEEP_MAGIC;
    s_keep.sum = sum_of(&s_keep);
}

// strncpy is newlib's and may lie in flash: the copy is a loop of its own.
static IRAM_ATTR void copy_text(char *into, size_t room, const char *from)
{
    size_t n = 0;
    if (from != NULL) {
        while (n < room - 1 && from[n] != '\0') {
            into[n] = from[n];
            n++;
        }
    }
    into[n] = '\0';
}

// Walks the stack a core was on and keeps the return addresses into TRACE, counting them in DEPTH - the same walk
// the panic handler makes to print a backtrace (esp_backtrace_print_from_frame of components/esp_system).
static IRAM_ATTR void walk(const XtExcFrame *exc, uint32_t *trace, uint32_t *depth)
{
    *depth = 0;
    if (exc == NULL) {
        return;
    }
    esp_backtrace_frame_t frame = {
        .pc = exc->pc, .sp = exc->a1, .next_pc = exc->a0, .exc_frame = exc,
    };
    trace[(*depth)++] = esp_cpu_process_stack_pc(frame.pc);
    // The first frame of an InstrFetchProhibited carries the very address that could not be fetched, so a PC
    // that is not executable is the answer there rather than a sign of a stack nobody can walk.
    if (!esp_stack_ptr_is_sane(frame.sp) ||
        (!esp_ptr_executable((void *)esp_cpu_process_stack_pc(frame.pc)) &&
         exc->exccause != EXCCAUSE_INSTR_PROHIBITED)) {
        return;
    }
    // A frame the walk itself calls unsound is where the note stops. Keeping it would put an address into the
    // trace that points at nothing, and an address in a trace reads as a place in our code.
    while (frame.next_pc != 0 && *depth < PANIC_NOTE_TRACE) {
        if (!esp_backtrace_get_next_frame(&frame)) {
            return;
        }
        trace[(*depth)++] = esp_cpu_process_stack_pc(frame.pc);
    }
}

// The other core's stack at the fall. Every core that reaches the panic handler leaves its frame in g_exc_frames
// before one of them goes on (esp_system/port/panic_handler.c) - on an interrupt watchdog the other core parks in
// busy_wait with its frame there, and it is that frame ESP-IDF itself prints as the other core's backtrace. A core
// that never reached the handler left NULL, and the walk keeps nothing: "no stack", not a guess.
static IRAM_ATTR void walk_other(const panic_info_t *info)
{
#if SOC_CPU_CORES_NUM > 1
    uint32_t other = info->core == 0 ? 1 : 0;
    s_keep.note.other_core = other;
    walk((const XtExcFrame *)g_exc_frames[other], s_keep.note.other_trace, &s_keep.note.other_depth);
#else
    (void)info;
    s_keep.note.other_core = PANIC_NOTE_NO_CORE;
    s_keep.note.other_depth = 0;
#endif
}

void panic_note_said(const char *text)
{
    // Stamping the start here is what makes the text survive the write below: our code speaks first, on its
    // way to abort(), and the chip's numbers arrive after it.
    s_keep.note.seq = s_keep.starts;
    copy_text(s_keep.note.said, PANIC_NOTE_SAID, text);
    reseal();
}

// The text the fall was handed: copied when it lies inside, kept by its address when it lies in flash - IDF's
// reasons are literals of whichever component fell, and a literal in flash is unreadable while the chip erases.
static IRAM_ATTR void keep_reason(const char *from)
{
    s_keep.note.reason_at = 0;
    if (from != NULL && !esp_ptr_internal(from)) {
        s_keep.note.reason[0] = '\0';
        s_keep.note.reason_at = (uint32_t)from;
        return;
    }
    copy_text(s_keep.note.reason, PANIC_NOTE_REASON, from);
}

static IRAM_ATTR void write_note(const panic_info_t *info)
{
    if (s_keep.note.seq != s_keep.starts) {
        // Nothing our code said this start, so whatever stands in said is an older fall's. A hardware fault
        // never goes through our code, and last week's message next to this week's address is a wrong answer.
        s_keep.note.said[0] = '\0';
    }
    s_keep.note.seq = s_keep.starts;
    s_keep.note.image = s_keep.image;
    s_keep.note.core = (uint32_t)info->core;
    s_keep.note.addr = (uint32_t)info->addr;
    s_keep.note.excvaddr = info->frame != NULL ? ((const XtExcFrame *)info->frame)->excvaddr : 0;
    // The same override esp_panic_handler makes right after this, and the note has to make it too: our code
    // giving up reaches the chip as an illegal instruction, so the arch has already filled the info in with a
    // fault. Recorded raw, every Rust panic would read as our code touching a bad address - the other repair.
    if (g_panic_abort) {
        s_keep.note.exception = PANIC_EXCEPTION_ABORT;
        keep_reason(g_panic_abort_details);
    } else {
        s_keep.note.exception = (uint32_t)info->exception;
        keep_reason(info->reason);
    }
    walk((const XtExcFrame *)info->frame, s_keep.note.trace, &s_keep.note.depth);
    walk_other(info);
    reseal();
}

void __real_esp_panic_handler(panic_info_t *info);

void IRAM_ATTR __wrap_esp_panic_handler(panic_info_t *info)
{
    write_note(info);
    __real_esp_panic_handler(info);
}

const panic_note_t *panic_note_rise(void)
{
    if (s_keep.magic != KEEP_MAGIC || s_keep.sum != sum_of(&s_keep)) {
        memset(&s_keep, 0, sizeof(s_keep));
    }
    s_handed = s_keep.note;
    // the reason the fall kept by its address: read now, with the cache on, and only from the image that fell - in
    // another image the address holds something else
    if (s_handed.reason_at != 0 && s_handed.image == panic_note_image() && esp_ptr_in_drom((const void *)s_handed.reason_at)) {
        copy_text(s_handed.reason, PANIC_NOTE_REASON, (const char *)s_handed.reason_at);
    }
    s_keep.starts++;
    // Taken here, where the cache is on and reading the image's description is an ordinary read, rather than
    // in the panic handler: a fall copies the number from this field and touches no flash.
    s_keep.image = panic_note_image();
    reseal();
    return s_handed.seq != 0 ? &s_handed : NULL;
}

uint32_t panic_note_starts(void)
{
    return s_keep.starts;
}

uint32_t panic_note_image(void)
{
    const esp_app_desc_t *desc = esp_app_get_description();
    uint32_t id = 0;
    if (desc != NULL) {
        memcpy(&id, desc->app_elf_sha256, sizeof(id));
    }
    return id;
}
