#[macro_use]
mod mooneye_macro;

#[test]
fn make_intellij_see_this_as_test_file() {}

mooneye_tests!(
    "timer",
    "acceptance/timer/",
    "div_write",
    // "rapid_toggle",
    "tim00",
    "tim00_div_trigger",
    "tim01",
    "tim01_div_trigger",
    "tim10",
    "tim10_div_trigger",
    "tim11",
    "tim11_div_trigger",
    // "tima_reload",
    // "tima_write_reloading",
    // "tma_write_reloading",
);

mooneye_tests!("instructions", "acceptance/instr/", "daa");

mooneye_tests!(
    "instruction_timing",
    "acceptance/",
    "div_timing",
    "pop_timing",
    "reti_intr_timing"
);

mooneye_tests!("bits", "acceptance/bits/", "reg_f");
