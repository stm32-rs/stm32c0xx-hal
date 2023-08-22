//! Quadrature Encoder Interface
use crate::gpio::{alt::TimCPin as CPin, PushPull};
use crate::pac;
use crate::rcc::{self, Rcc};

pub trait QeiExt: Sized + Instance {
    fn qei(
        self,
        pins: (
            impl Into<<Self as CPin<0>>::Ch<PushPull>>,
            impl Into<<Self as CPin<1>>::Ch<PushPull>>,
        ),
        rcc: &mut Rcc,
    ) -> Qei<Self>;
}

impl<TIM: Instance> QeiExt for TIM {
    fn qei(
        self,
        pins: (
            impl Into<<Self as CPin<0>>::Ch<PushPull>>,
            impl Into<<Self as CPin<1>>::Ch<PushPull>>,
        ),
        rcc: &mut Rcc,
    ) -> Qei<Self> {
        Qei::new(self, pins, rcc)
    }
}

/// Hardware quadrature encoder interface peripheral
pub struct Qei<TIM: Instance> {
    tim: TIM,
    pins: (
        <TIM as CPin<0>>::Ch<PushPull>,
        <TIM as CPin<1>>::Ch<PushPull>,
    ),
}

impl<TIM: Instance> Qei<TIM> {
    /// Configures a TIM peripheral as a quadrature encoder interface input
    pub fn new(
        mut tim: TIM,
        pins: (
            impl Into<<TIM as CPin<0>>::Ch<PushPull>>,
            impl Into<<TIM as CPin<1>>::Ch<PushPull>>,
        ),
        rcc: &mut Rcc,
    ) -> Self {
        // enable and reset peripheral to a clean slate state
        TIM::enable(rcc);
        TIM::reset(rcc);

        tim.setup_qei();
        let pins = (pins.0.into(), pins.1.into());
        tim.start();

        Qei { tim, pins }
    }

    /// Releases the TIM peripheral and QEI pins
    #[allow(clippy::type_complexity)]
    pub fn release(
        self,
    ) -> (
        TIM,
        (
            <TIM as CPin<0>>::Ch<PushPull>,
            <TIM as CPin<1>>::Ch<PushPull>,
        ),
    ) {
        (self.tim, self.pins)
    }
}

pub trait Instance: crate::Sealed + rcc::Enable + rcc::Reset + CPin<0> + CPin<1> {
    fn setup_qei(&mut self);
    fn start(&mut self);
    fn read_direction(&self) -> bool;
}

macro_rules! hal {
    ($TIM:ty) => {
        impl embedded_hal::Qei for Qei<$TIM> {
            type Count = u16;

            fn count(&self) -> u16 {
                self.tim.cnt.read().bits() as u16
            }

            fn direction(&self) -> embedded_hal::Direction {
                if self.tim.read_direction() {
                    embedded_hal::Direction::Upcounting
                } else {
                    embedded_hal::Direction::Downcounting
                }
            }
        }

        impl Instance for $TIM {
            fn setup_qei(&mut self) {
                // Configure TxC1 and TxC2 as captures
                self.ccmr1_output()
                    .write(|w| unsafe { w.cc1s().bits(0b01).cc2s().bits(0b01) });

                // Encoder mode 2.
                self.smcr.write(|w| unsafe { w.sms1().bits(0b010) });

                // Enable and configure to capture on rising edge
                self.ccer.write(|w| {
                    w.cc1e().set_bit();
                    w.cc2e().set_bit();
                    w.cc1p().clear_bit();
                    w.cc2p().clear_bit();
                    w.cc1np().clear_bit();
                    w.cc2np().clear_bit()
                });
            }

            fn start(&mut self) {
                self.cr1.write(|w| w.cen().set_bit());
            }

            fn read_direction(&self) -> bool {
                self.cr1.read().dir().bit_is_clear()
            }
        }
    };
}

hal! { pac::TIM1 }
hal! { pac::TIM3 }
