#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate rtic;
extern crate stm32c0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::*;
use hal::prelude::*;
use hal::stm32;
use hal::time::*;
use hal::timer::*;

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        timer: Timer<stm32::TIM17>,
    }

    #[local]
    struct Local {
        exti: stm32::EXTI,
        led: PA5<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.constrain();
        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let mut timer = ctx.device.TIM17.timer(&mut rcc);
        timer.start(Hertz::Hz(3).into_duration());
        timer.listen();

        let mut exti = ctx.device.EXTI;
        gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        (
            Shared { timer },
            Local {
                exti,
                led: gpioa.pa5.into_push_pull_output(),
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM17, shared = [timer], local = [led])]
    fn timer_tick(mut ctx: timer_tick::Context) {
        ctx.local.led.toggle();
        ctx.shared.timer.lock(|tim| tim.clear_irq());
    }

    #[task(binds = EXTI4_15, shared = [timer], local = [exti])]
    fn button_click(mut ctx: button_click::Context) {
        ctx.shared.timer.lock(|tim| {
            if tim.enabled() {
                tim.pause();
            } else {
                tim.resume();
            }
        });
        ctx.local.exti.unpend(Event::GPIO13);
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
