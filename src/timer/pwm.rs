//! # Pulse Width Modulation
use core::marker::PhantomData;

use crate::rcc::*;
use crate::stm32::*;
use crate::time::Hertz;
use crate::timer::pins::TimerPin;
use crate::timer::*;

pub enum OutputCompareMode {
    Frozen = 0,
    MatchPos = 1,
    MatchNeg = 2,
    MatchToggle = 3,
    ForceLow = 4,
    ForceHigh = 5,
    PwmMode1 = 6,
    PmwMode2 = 7,
    OpmMode1 = 8,
    OomMode2 = 9,
    CombinedMode1 = 12,
    CombinedMode2 = 13,
    AsyncMode1 = 14,
    AsyncMode2 = 15,
}

pub struct Pwm<TIM> {
    clk: Hertz,
    tim: TIM,
}

pub struct PwmPin<TIM, CH> {
    tim: PhantomData<TIM>,
    channel: PhantomData<CH>,
}

pub trait PwmExt: Sized {
    fn pwm(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self>;
}

pub trait PwmPinMode {
    fn set_compare_mode(&mut self, mode: OutputCompareMode);
}

impl<TIM> Pwm<TIM> {
    pub fn bind_pin<PIN>(&self, pin: PIN) -> PwmPin<TIM, PIN::Channel>
    where
        PIN: TimerPin<TIM>,
    {
        pin.setup();
        PwmPin {
            tim: PhantomData,
            channel: PhantomData,
        }
    }
}

macro_rules! pwm {
    ($($TIMX:ident: ($timX:ident, $arr:ident $(,$arr_h:ident)*),)+) => {
        $(
            impl PwmExt for $TIMX {
                fn pwm(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self> {
                    $timX(self, freq, rcc)
                }
            }

            fn $timX(tim: $TIMX, freq: Hertz, rcc: &mut Rcc) -> Pwm<$TIMX> {
                $TIMX::enable(rcc);
                $TIMX::reset(rcc);

                let clk = rcc.clocks.apb_tim_clk;
                let mut pwm = Pwm::<$TIMX> {
                    clk,
                    tim,
                };
                pwm.set_freq(freq);
                pwm
            }

            impl Pwm<$TIMX> {
                /// Set the PWM frequency. Actual frequency may differ from
                /// requested due to precision of input clock. To check actual
                /// frequency, call freq.
                pub fn set_freq(&mut self, freq: Hertz) {
                    let ratio = self.clk / freq;
                    let psc = (ratio - 1) / 0xffff;
                    let arr = ratio / (psc + 1) - 1;

                    unsafe {
                        self.tim.psc().write(|w| w.psc().bits(psc as u16));
                        self.tim.arr().write(|w| w.$arr().bits((arr as u16).into()));
                        $(
                            self.tim.arr().modify(|_, w| w.$arr_h().bits((arr >> 16) as u16));
                        )*
                        self.tim.cr1().write(|w| w.cen().set_bit());
                    }
                }
                /// Starts listening
                pub fn listen(&mut self) {
                    self.tim.dier().write(|w| w.uie().set_bit());
                }

                /// Stops listening
                pub fn unlisten(&mut self) {
                    self.tim.dier().write(|w| w.uie().clear_bit());
                }
                /// Clears interrupt flag
                pub fn clear_irq(&mut self) {
                    self.tim.sr().modify(|_, w| w.uif().clear_bit());
                }

                /// Resets counter value
                pub fn reset(&mut self) {
                    self.tim.cnt().reset();
                }

                /// Returns the currently configured frequency
                pub fn freq(&self) -> Hertz {
                    Hertz::from_raw(self.clk.raw()
                        / (self.tim.psc().read().bits() as u32 + 1)
                        / (self.tim.arr().read().bits() as u32 + 1))
                }
            }
        )+
    }
}

#[allow(unused_macros)]
macro_rules! pwm_q {
    ($($TIMX:ident: $timX:ident,)+) => {
        $(
            impl PwmQExt for $TIMX {
                fn pwm_q(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self> {
                    $timX(self, freq, rcc, ClockSource::Pllq)
                }
            }
        )+
    }
}

macro_rules! pwm_hal {
    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxpe:ident, $ocxm:ident, $ccrx:ident, $ccrx_l:ident, $ccrx_h:ident),)+
    ) => {
        $(
            impl hal::PwmPin for PwmPin<$TIMX, $CH> {
                type Duty = u32;

                fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer().modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer().modify(|_, w| w.$ccxe().set_bit());
                    }
                }

                fn get_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).$ccrx().read().bits() }
                }

                fn get_max_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).arr().read().bits() }
                }

                fn set_duty(&mut self, duty: u32) {
                    unsafe { (*$TIMX::ptr()).$ccrx().write(|w| w.bits(duty)) };
                }
            }
        )+
    };
}

macro_rules! pwm_advanced_hal {
    ($($TIMX:ident: (
        $CH:ty,
        $ccxe:ident $(: $ccxne:ident)*,
        $ccmrx_output:ident,
        $ocxpe:ident,
        $ocxm:ident,
        $ccrx:expr
        $(, $moe:ident)*
    ) ,)+
    ) => {
        $(
            impl hal::PwmPin for PwmPin<$TIMX, $CH> {
                type Duty = u16;

                fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer().modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer().modify(|_, w| w.$ccxe().set_bit());
                        $(
                            tim.ccer().modify(|_, w| w.$ccxne().bit(true));
                        )*
                        $(
                            tim.bdtr().modify(|_, w| w.$moe().set_bit());
                        )*
                    }
                }

                fn get_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).ccr($ccrx).read().ccr().bits() }
                }

                fn get_max_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).arr().read().arr().bits() }
                }

                fn set_duty(&mut self, duty: u16) {
                    unsafe { (*$TIMX::ptr()).ccr($ccrx).write(|w| w.ccr().bits(duty)) };
                }
            }

            impl PwmPinMode for PwmPin<$TIMX, $CH>{
                fn set_compare_mode(&mut self, mode: OutputCompareMode) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxm().bits(mode as u8));
                    }
                }
            }
        )+
    };
}

pwm_advanced_hal! {
    TIM1:  (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, 1, moe),
    TIM1:  (Channel2, cc2e: cc2ne, ccmr1_output, oc2pe, oc2m, 2, moe),
    TIM1:  (Channel3, cc3e: cc3ne, ccmr2_output, oc3pe, oc3m, 3, moe),
    TIM1:  (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, 4, moe),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, 1),
    TIM16: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, 1, moe),
    TIM17: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, 1, moe),
}

pwm_hal! {
    TIM3: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, ccr1_l, ccr1_h),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2, ccr2_l, ccr2_h),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3, ccr3_l, ccr3_h),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, ccr4_l, ccr4_h),
}

pwm! {
    TIM1: (tim1, arr),
    TIM3: (tim3, arr),
    TIM14: (tim14, arr),
    TIM16: (tim16, arr),
    TIM17: (tim17, arr),
}
