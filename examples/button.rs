#![no_std]
#![no_main]
#![deny(warnings)]
#![deny(unsafe_code)]

extern crate panic_halt;
extern crate stm32c0xx_hal as hal;

use cortex_m_rt::entry;
use hal::prelude::*;
use hal::stm32;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let port_a = dp.GPIOA.split(&mut rcc);
    let port_c = dp.GPIOC.split(&mut rcc);

    let button = port_c.pc13.into_pull_up_input();
    let mut led = port_a.pa5.into_push_pull_output();

    loop {
        if button.is_high().unwrap() {
            led.set_low().unwrap();
        } else {
            led.set_high().unwrap();
        }
    }
}
