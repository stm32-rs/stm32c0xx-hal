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
            apb_clk: 12.MHz(),
            apb_tim_clk: 12.MHz(),
            core_clk: 1_500.kHz(),
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
    pub fn freeze(self, cfg: Config) -> Self {
        let (sys_clk, sw_bits) = match cfg.sys_mux {
            SysClockSrc::HSE(freq) => {
                self.enable_hse(false);
                (freq, 0b001)
            }
            SysClockSrc::HSE_BYPASS(freq) => {
                self.enable_hse(true);
                (freq, 0b001)
            }
            SysClockSrc::LSE(freq) => {
                self.enable_lse(false);
                (freq, 0b100)
            }
            SysClockSrc::LSE_BYPASS(freq) => {
                self.enable_lse(true);
                (freq, 0b100)
            }
            SysClockSrc::LSI => {
                self.enable_lsi();
                (32_768.Hz(), 0b011)
            }
            SysClockSrc::HSI(prs) => {
                self.enable_hsi();
                let (freq, div_bits) = match prs {
                    Prescaler::Div2 => (HSI_FREQ / 2, 0b001),
                    Prescaler::Div4 => (HSI_FREQ / 4, 0b010),
                    Prescaler::Div8 => (HSI_FREQ / 8, 0b011),
                    Prescaler::Div16 => (HSI_FREQ / 16, 0b100),
                    Prescaler::Div32 => (HSI_FREQ / 32, 0b101),
                    Prescaler::Div64 => (HSI_FREQ / 64, 0b110),
                    Prescaler::Div128 => (HSI_FREQ / 128, 0b111),
                    _ => (HSI_FREQ, 0b000),
                };
                self.cr.write(|w| w.hsidiv().bits(div_bits));
                (freq.Hz(), 0b000)
            }
        };

        let sys_freq = sys_clk.raw();
        let (ahb_freq, ahb_psc_bits) = match cfg.ahb_psc {
            Prescaler::Div2 => (sys_freq / 2, 0b1000),
            Prescaler::Div4 => (sys_freq / 4, 0b1001),
            Prescaler::Div8 => (sys_freq / 8, 0b1010),
            Prescaler::Div16 => (sys_freq / 16, 0b1011),
            Prescaler::Div64 => (sys_freq / 64, 0b1100),
            Prescaler::Div128 => (sys_freq / 128, 0b1101),
            Prescaler::Div256 => (sys_freq / 256, 0b1110),
            Prescaler::Div512 => (sys_freq / 512, 0b1111),
            _ => (sys_clk.raw(), 0b0000),
        };
        let (apb_freq, apb_tim_freq, apb_psc_bits) = match cfg.apb_psc {
            Prescaler::Div2 => (ahb_freq / 2, ahb_freq, 0b100),
            Prescaler::Div4 => (ahb_freq / 4, ahb_freq / 2, 0b101),
            Prescaler::Div8 => (ahb_freq / 8, ahb_freq / 4, 0b110),
            Prescaler::Div16 => (ahb_freq / 16, ahb_freq / 8, 0b111),
            _ => (ahb_freq, ahb_freq, 0b000),
        };

        self.cfgr.modify(|_, w| unsafe {
            w.hpre()
                .bits(ahb_psc_bits)
                .ppre()
                .bits(apb_psc_bits)
                .sw()
                .bits(sw_bits)
        });

        while self.cfgr.read().sws().bits() != sw_bits {}

        Rcc {
            rb: self.rb,
            clocks: Clocks {
                sys_clk,
                ahb_clk: ahb_freq.Hz(),
                apb_clk: apb_freq.Hz(),
                apb_tim_clk: apb_tim_freq.Hz(),
                core_clk: (ahb_freq / 8).Hz(),
            },
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
        self.csr2.modify(|_, w| w.lsion().set_bit());
        while self.csr2.read().lsirdy().bit_is_clear() {}
    }

    pub(crate) fn enable_lse(&self, bypass: bool) {
        self.csr1
            .modify(|_, w| w.lseon().set_bit().lsebyp().bit(bypass));
        while self.csr1.read().lserdy().bit_is_clear() {}
    }

    pub(crate) fn enable_pwr_clock(&self) {
        self.apbenr1.modify(|_, w| w.pwren().set_bit());
    }

    pub(crate) fn enable_rtc(&self, src: RTCSrc) {
        self.enable_pwr_clock();
        self.apbenr1
            .modify(|_, w| w.rtcapben().set_bit().pwren().set_bit());
        self.apbsmenr1.modify(|_, w| w.rtcapbsmen().set_bit());
        self.csr1.modify(|_, w| w.rtcrst().set_bit());
        let rtc_sel = match src {
            RTCSrc::LSE | RTCSrc::LSE_BYPASS => 0b01,
            RTCSrc::LSI => 0b10,
            RTCSrc::HSE | RTCSrc::HSE_BYPASS => 0b11,
        };

        self.csr1.modify(|_, w| {
            w.rtcsel()
                .bits(rtc_sel)
                .rtcen()
                .set_bit()
                .rtcrst()
                .clear_bit()
        });

        match src {
            RTCSrc::LSE => self.enable_lse(false),
            RTCSrc::LSE_BYPASS => self.enable_lse(true),
            RTCSrc::LSI => self.enable_lsi(),
            RTCSrc::HSE => self.enable_hse(false),
            RTCSrc::HSE_BYPASS => self.enable_hse(true),
        };
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
