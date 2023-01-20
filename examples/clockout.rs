#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32c0xx_hal as hal;

use hal::gpio::Speed;
use hal::prelude::*;
use hal::rcc::{Config, LSCOSrc, MCOSrc, Prescaler};
use hal::stm32;
use rt::entry;

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.freeze(Config::hsi(Prescaler::NotDivided));
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut mco =
        gpioa
            .pa9
            .set_speed(Speed::VeryHigh)
            .mco(MCOSrc::SysClk, Prescaler::Div2, &mut rcc);
    mco.enable();

    let mut lsco = gpioa.pa2.lsco(LSCOSrc::LSE, &mut rcc);
    lsco.enable();

    loop {}
}
