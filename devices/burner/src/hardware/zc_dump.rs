//! HERE: what the chip holds for the zero-crossing pin, READ and not set: the IDF's own dump of the pin's configuration, and the words of the registers the dump leaves out (the interrupt's type and enable, the pad's filter, the level).
//! NEVER HERE: a write to any register of the pin, the interrupt or the gate; what the fields mean (crates/dimmer, `zc_regs`); the counting (src/hardware/fan_gate.rs).
//! CHECKS: crates/dimmer/tests/zc_regs.rs for the fields; on the board, the line of `zc` against the setup in fan_gate.rs (`start`).

use std::ptr::read_volatile;

use esp_idf_svc::sys;

/// The zero-crossing pin, burner.yaml's.
fn pin() -> u32 {
    super::pin(env!("STRAGA_PIN_ZC")) as u32
}

/// The registers of the pin read into fields: IO_MUX_GPIOn_REG at the base plus 4 plus 4 a pin (the way `fan_gate` finds the pad's filter), GPIO_PINn_REG at four a pin from GPIO_PIN0_REG, and the pin's bit of the input word.
pub fn registers() -> dimmer::ZcRegs {
    let zc = pin();
    let io_mux = sys::REG_IO_MUX_BASE + 4 + zc * 4;
    let pin_reg = sys::GPIO_PIN0_REG + zc * 4;
    let (level_reg, bit) = if zc < 32 { (sys::GPIO_IN_REG, zc) } else { (sys::GPIO_IN1_REG, zc - 32) };
    // each read is one load of a register that nothing here writes
    unsafe { dimmer::zc_regs(read_volatile(io_mux as *const u32), read_volatile(pin_reg as *const u32), read_volatile(level_reg as *const u32), bit) }
}

/// What the IDF prints of the pin's configuration, line by line to LINE. It prints to a stream; the stream is a buffer that lives for this call and is freed before it returns.
pub fn dump(line: &mut dyn FnMut(&str)) -> Result<(), String> {
    let mut buffer: *mut core::ffi::c_char = core::ptr::null_mut();
    let mut size: usize = 0;
    let stream = unsafe { sys::open_memstream(&mut buffer, &mut size) };
    if stream.is_null() {
        return Err("no stream to dump the pin's configuration into".into());
    }
    let dumped = unsafe { sys::gpio_dump_io_configuration(stream, 1u64 << pin()) };
    // closing the stream is what completes BUFFER and SIZE
    unsafe { sys::fclose(stream) };
    if !buffer.is_null() {
        let text = unsafe { std::slice::from_raw_parts(buffer as *const u8, size) };
        String::from_utf8_lossy(text).lines().filter(|l| !l.is_empty()).for_each(|l| line(l));
        unsafe { sys::free(buffer as *mut core::ffi::c_void) };
    }
    sys::esp!(dumped).map_err(|e| format!("gpio_dump_io_configuration: {e}"))
}
