#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;
extern crate stm32c0xx_hal as hal;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

#[entry]
fn main() -> ! {
    hprintln!("Hello, STM32C0!").unwrap();

    loop {
        cortex_m::asm::nop();
    }
}
