#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting;
extern crate panic_halt;
extern crate stm32c0xx_hal as hal;

use hal::prelude::*;
use hal::rcc::Config;
use hal::stm32;
use rt::entry;

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.freeze(Config::hse(48.MHz()));

    let port_a = dp.GPIOA.split(&mut rcc);
    let mut led = port_a.pa5.into_push_pull_output();

    let mut watchdog = dp.WWDG.constrain(&mut rcc);
    // let mut watchdog = dp.IWDG.constrain();

    led.set_high().ok();
    watchdog.start(20.millis());

    loop {}
}
