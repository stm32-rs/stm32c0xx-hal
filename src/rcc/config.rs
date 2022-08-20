use crate::time::Hertz;

/// Prescaler
#[derive(Clone, Copy)]
pub enum Prescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
    Div32,
    Div64,
    Div128,
    Div256,
    Div512,
}

/// System clock mux source
pub enum SysClockSrc {
    HSI(Prescaler),
    HSE(Hertz),
    HSE_BYPASS(Hertz),
}

/// Microcontroller clock output source
pub enum MCOSrc {
    LSI,
    PLL,
    SysClk,
    HSI,
    HSE,
    LSE,
}

/// Low-speed clocks output source
pub enum LSCOSrc {
    LSI,
    LSE,
}

/// RTC clock input source
#[derive(Clone, Copy)]
pub enum RTCSrc {
    LSE = 0b01,
    LSI = 0b10,
    HSE = 0b11,
}

/// Clocks configutation
pub struct Config {
    pub(crate) sys_mux: SysClockSrc,
    pub(crate) ahb_psc: Prescaler,
    pub(crate) apb_psc: Prescaler,
}

impl Config {
    pub fn new(mux: SysClockSrc) -> Self {
        Config::default().clock_src(mux)
    }

    pub fn hsi(psc: Prescaler) -> Self {
        Config::default().clock_src(SysClockSrc::HSI(psc))
    }

    pub fn clock_src(mut self, mux: SysClockSrc) -> Self {
        self.sys_mux = mux;
        self
    }

    pub fn ahb_psc(mut self, psc: Prescaler) -> Self {
        self.ahb_psc = psc;
        self
    }

    pub fn apb_psc(mut self, psc: Prescaler) -> Self {
        self.apb_psc = psc;
        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            sys_mux: SysClockSrc::HSI(Prescaler::NotDivided),
            ahb_psc: Prescaler::NotDivided,
            apb_psc: Prescaler::NotDivided,
        }
    }
}
