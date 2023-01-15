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
    let mut led = port_a.pa0.into_push_pull_output();
    loop {
        led.toggle().unwrap();
        for _ in 0..1_000_000 {
            cortex_m::asm::nop();
        }
    }
}
