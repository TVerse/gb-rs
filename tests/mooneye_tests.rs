#[macro_use]
mod mooneye_macro;

mooneye_tests!(
    "timer",
    "acceptance/timer/",
    "div_write",
    "rapid_toggle",
    "tim00",
    "tim00_div_trigger",
    "tim01",
    "tim01_div_trigger",
    "tim10",
    "tim10_div_trigger",
    "tim11",
    "tim11_div_trigger",
    "tima_reload",
    "tima_write_reloading",
    "tma_write_reloading",
);
