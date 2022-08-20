use crate::stm32::{rcc, RCC};
use crate::time::Hertz;
use fugit::RateExtU32;

mod clockout;
mod config;
mod enable;

pub use clockout::*;
pub use config::*;

/// HSI frequency
pub const HSI_FREQ: u32 = 48_000_000;

/// Clock frequencies
#[derive(Clone, Copy)]
pub struct Clocks {
    /// System frequency
    pub sys_clk: Hertz,
    /// Core frequency
    pub core_clk: Hertz,
    /// AHB frequency
    pub ahb_clk: Hertz,
    /// APB frequency
    pub apb_clk: Hertz,
    /// APB timers frequency
    pub apb_tim_clk: Hertz,
}

impl Default for Clocks {
    fn default() -> Clocks {
        Clocks {
            sys_clk: 12.MHz(),
            ahb_clk: 12.MHz(),
            core_clk: 12.MHz(),
            apb_clk: 12.MHz(),
            apb_tim_clk: 12.MHz(),
        }
    }
}

/// Constrained RCC peripheral
pub struct Rcc {
    /// Clock configuration
    pub clocks: Clocks,
    pub(crate) rb: RCC,
}

impl core::ops::Deref for Rcc {
    type Target = RCC;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.rb
    }
}

impl Rcc {
    /// Apply clock configuration
    pub fn freeze(self, _cfg: Config) -> Self {
        Rcc {
            rb: self.rb,
            clocks: Clocks::default(),
        }
    }

    pub(crate) fn enable_hsi(&self) {
        self.cr.modify(|_, w| w.hsion().set_bit());
        while self.cr.read().hsirdy().bit_is_clear() {}
    }

    pub(crate) fn enable_hse(&self, bypass: bool) {
        self.cr
            .modify(|_, w| w.hseon().set_bit().hsebyp().bit(bypass));
        while self.cr.read().hserdy().bit_is_clear() {}
    }

    pub(crate) fn enable_lsi(&self) {
        // self.csr.modify(|_, w| w.lsion().set_bit());
        // while self.csr.read().lsirdy().bit_is_clear() {}
        todo!();
    }

    pub(crate) fn enable_lse(&self, _bypass: bool) {
        // self.bdcr
        //     .modify(|_, w| w.lseon().set_bit().lsebyp().bit(bypass));
        // while self.bdcr.read().lserdy().bit_is_clear() {}

        todo!();
    }

    pub(crate) fn unlock_rtc(&self) {
        self.apbenr1.modify(|_, w| w.pwren().set_bit());
        // let pwr = unsafe { &(*crate::stm32::PWR::ptr()) };
        // pwr.cr1.modify(|_, w| w dbp().set_bit());
        // while pwr.cr1.read().dbp().bit_is_clear() {}
    }

    pub(crate) fn enable_rtc(&self, src: RTCSrc) {
        match src {
            RTCSrc::LSI => self.enable_lsi(),
            RTCSrc::HSE => self.enable_hse(false),
            RTCSrc::LSE => self.enable_lse(false),
        }
        self.apbenr1
            .modify(|_, w| w.rtcapben().set_bit().pwren().set_bit());
        self.apbsmenr1.modify(|_, w| w.rtcapbsmen().set_bit());
        self.unlock_rtc();
        // self.bdcr.modify(|_, w| w.bdrst().set_bit());
        // self.bdcr.modify(|_, w| unsafe {
        //     w.rtcsel()
        //         .bits(src as u8)
        //         .rtcen()
        //         .set_bit()
        //         .bdrst()
        //         .clear_bit()
        // });
    }
}

/// Extension trait that constrains the `RCC` peripheral
pub trait RccExt {
    /// Constrains the `RCC` peripheral so it plays nicely with the other abstractions
    fn constrain(self) -> Rcc;
    /// Constrains the `RCC` peripheral and apply clock configuration
    fn freeze(self, rcc_cfg: Config) -> Rcc;
}

impl RccExt for RCC {
    fn constrain(self) -> Rcc {
        Rcc {
            rb: self,
            clocks: Clocks::default(),
        }
    }

    fn freeze(self, rcc_cfg: Config) -> Rcc {
        self.constrain().freeze(rcc_cfg)
    }
}

/// Bus associated to peripheral
pub trait RccBus: crate::Sealed {
    /// Bus type;
    type Bus;
}

/// Enable/disable peripheral
pub trait Enable: RccBus {
    /// Enables peripheral
    fn enable(rcc: &mut Rcc);

    /// Disables peripheral
    fn disable(rcc: &mut Rcc);

    /// Check if peripheral enabled
    fn is_enabled() -> bool;

    /// Check if peripheral disabled
    fn is_disabled() -> bool;

    /// # Safety
    ///
    /// Enables peripheral. Takes access to RCC internally
    unsafe fn enable_unchecked();

    /// # Safety
    ///
    /// Disables peripheral. Takes access to RCC internally
    unsafe fn disable_unchecked();
}

/// Enable/disable peripheral in Sleep mode
pub trait SMEnable: RccBus {
    /// Enables peripheral
    fn sleep_mode_enable(rcc: &mut Rcc);

    /// Disables peripheral
    fn sleep_mode_disable(rcc: &mut Rcc);

    /// Check if peripheral enabled
    fn is_sleep_mode_enabled() -> bool;

    /// Check if peripheral disabled
    fn is_sleep_mode_disabled() -> bool;

    /// # Safety
    ///
    /// Enables peripheral. Takes access to RCC internally
    unsafe fn sleep_mode_enable_unchecked();

    /// # Safety
    ///
    /// Disables peripheral. Takes access to RCC internally
    unsafe fn sleep_mode_disable_unchecked();
}

/// Reset peripheral
pub trait Reset: RccBus {
    /// Resets peripheral
    fn reset(rcc: &mut Rcc);

    /// # Safety
    ///
    /// Resets peripheral. Takes access to RCC internally
    unsafe fn reset_unchecked();
}

use crate::stm32::rcc::RegisterBlock as RccRB;

macro_rules! bus_struct {
    ($($busX:ident => ($EN:ident, $en:ident, $SMEN:ident, $smen:ident, $RST:ident, $rst:ident, $doc:literal),)+) => {
        $(
            #[doc = $doc]
            pub struct $busX {
                _0: (),
            }

            impl $busX {
                #[inline(always)]
                fn enr(rcc: &RccRB) -> &rcc::$EN {
                    &rcc.$en
                }

                #[inline(always)]
                fn smenr(rcc: &RccRB) -> &rcc::$SMEN {
                    &rcc.$smen
                }

                #[inline(always)]
                fn rstr(rcc: &RccRB) -> &rcc::$RST {
                    &rcc.$rst
                }
            }
        )+
    };
}

bus_struct! {
    AHB => (AHBENR, ahbenr, AHBSMENR, ahbsmenr, AHBRSTR, ahbrstr, "AMBA High-performance Bus (AHB) registers"),
    APB1 => (APBENR1, apbenr1, APBSMENR1, apbsmenr1, APBRSTR1, apbrstr1, "Advanced Peripheral Bus 1 (APB1) registers"),
    APB2 => (APBENR2, apbenr2, APBSMENR2, apbsmenr2, APBRSTR2, apbrstr2, "Advanced Peripheral Bus 2 (APB2) registers"),
    IOP => (IOPENR, iopenr, IOPSMENR, iopsmenr, IOPRSTR, ioprstr, "Input-Output Peripheral Bus (IOP) registers"),
}
