#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32c0xx_hal as hal;

use hal::i2c::Config;
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.freeze(rcc::Config::hsi(rcc::Prescaler::NotDivided));
    let mut delay = dp.TIM3.delay(&mut rcc);

    let gpiob = dp.GPIOB.split(&mut rcc);
    let sda = gpiob.pb9.into_open_drain_output_in_state(PinState::High);
    let scl = gpiob.pb8.into_open_drain_output_in_state(PinState::High);

    let mut i2c = dp.I2C.i2c((scl, sda), Config::new(400.kHz()), &mut rcc);

    i2c.write(0x2a, &[0x80, 0xff]).unwrap();
    i2c.write(0x2a, &[0x01, 0x04, 0x00, 0x00]).unwrap();

    let mut buf: [u8; 4] = [0, 0, 0, 0];
    loop {
        delay.delay(100.millis());
        buf[3] = (buf[3] + 1) % 24;
        i2c.write(0x2b, &buf).unwrap();
    }
}
