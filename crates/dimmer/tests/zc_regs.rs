//! The words of the zero-crossing pin's registers, read into fields. The places of the bits are the chip's (soc/esp32s3/register/soc/io_mux_reg.h and gpio_reg.h): the IO-MUX word has pull-down at bit 7, pull-up at 8, input enable at 9, the function in bits 12..14 and the pad filter at 15; the pin's own word has the interrupt type in bits 7..9 and the interrupt-enable mask in bits 13..17; the level is one bit of the input word.

use dimmer::zc_regs;

const NONE: u32 = 0;

#[test]
fn a_word_of_zeroes_says_nothing_is_set() {
    let r = zc_regs(NONE, NONE, NONE, 9);
    assert_eq!((r.func, r.int_type, r.int_ena), (0, 0, 0));
    assert!(!r.input_enable && !r.pull_up && !r.pull_down && !r.filter && !r.level);
}

#[test]
fn input_enable_is_bit_9_of_the_pad() {
    assert!(zc_regs(1 << 9, NONE, NONE, 9).input_enable);
    assert!(!zc_regs(!(1 << 9), NONE, NONE, 9).input_enable);
}

#[test]
fn pull_up_is_bit_8_and_pull_down_bit_7() {
    let up = zc_regs(1 << 8, NONE, NONE, 9);
    assert!(up.pull_up && !up.pull_down);
    let down = zc_regs(1 << 7, NONE, NONE, 9);
    assert!(down.pull_down && !down.pull_up);
}

#[test]
fn the_pad_filter_is_bit_15_and_nothing_next_to_it_is() {
    assert!(zc_regs(1 << 15, NONE, NONE, 9).filter);
    assert!(!zc_regs(1 << 14, NONE, NONE, 9).filter, "bit 14 is the top of the function");
    assert!(!zc_regs(!(1 << 15), NONE, NONE, 9).filter);
}

#[test]
fn the_function_is_bits_12_to_14() {
    assert_eq!(zc_regs(1 << 12, NONE, NONE, 9).func, 1);
    assert_eq!(zc_regs(0b111 << 12, NONE, NONE, 9).func, 7);
    assert_eq!(zc_regs(0b101 << 12 | 1 << 15 | 1 << 9, NONE, NONE, 9).func, 5, "the neighbours do not leak into it");
}

#[test]
fn the_interrupt_type_is_bits_7_to_9_of_the_pins_word() {
    assert_eq!(zc_regs(NONE, 3 << 7, NONE, 9).int_type, 3);
    assert_eq!(zc_regs(NONE, 5 << 7, NONE, 9).int_type, 5);
    assert_eq!(zc_regs(NONE, 1 << 10 | 1 << 13, NONE, 9).int_type, 0, "wakeup at bit 10 and the enable at 13 are not the type");
}

#[test]
fn the_interrupt_enable_is_the_mask_in_bits_13_to_17() {
    assert_eq!(zc_regs(NONE, 1 << 13, NONE, 9).int_ena, 1);
    assert_eq!(zc_regs(NONE, 0x1F << 13, NONE, 9).int_ena, 0x1F);
    assert_eq!(zc_regs(NONE, 3 << 7 | 1 << 12 | 1 << 18, NONE, 9).int_ena, 0, "the type, the config bits and the bit past the field are not the mask");
}

#[test]
fn the_level_is_the_named_bit_of_the_input_word() {
    assert!(zc_regs(NONE, NONE, 1 << 9, 9).level);
    assert!(!zc_regs(NONE, NONE, !(1 << 9), 9).level, "a high neighbour is not this pin");
    assert!(zc_regs(NONE, NONE, 1 << 3, 3).level);
}

#[test]
fn the_line_carries_every_field_and_the_two_counts_as_numbers() {
    let r = zc_regs(1 << 9 | 1 << 15 | 1 << 12, 3 << 7 | 1 << 13, 1 << 9, 9);
    assert_eq!(r.line(1200, 1198), "zc: func=1 ie=1 pu=0 pd=0 filter=1 int_type=3 int_ena=1 level=1 rising=1200 falling=1198");
}
