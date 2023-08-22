use crate::gpio::{self, alt::rcc as alt};
use crate::rcc::*;
use crate::stm32::RCC;

pub type LscoPin = gpio::PA2;

pub struct Lsco {
    pin: gpio::PA2<gpio::Alternate<0>>,
}

impl Lsco {
    pub fn enable(&mut self) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.csr1.modify(|_, w| w.lscoen().set_bit());
    }

    pub fn disable(&mut self) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.csr1.modify(|_, w| w.lscoen().clear_bit());
    }

    pub fn release(self) -> LscoPin {
        self.pin.into_mode()
    }
}

pub trait LSCOExt {
    fn lsco(self, src: LSCOSrc, rcc: &mut Rcc) -> Lsco;
}

impl LSCOExt for LscoPin {
    fn lsco(self, src: LSCOSrc, rcc: &mut Rcc) -> Lsco {
        let src_select_bit = match src {
            LSCOSrc::LSE => {
                rcc.enable_lse(false);
                true
            }
            LSCOSrc::LSI => {
                rcc.enable_lsi();
                false
            }
        };
        let pin = self.into_mode();
        rcc.csr1.modify(|_, w| w.lscosel().bit(src_select_bit));
        Lsco { pin }
    }
}

pub struct Mco {
    pin: alt::Mco,
    src_bits: u8,
}

impl Mco {
    pub fn enable(&mut self) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.cfgr
            .modify(|_, w| unsafe { w.mcosel().bits(self.src_bits) });
    }

    pub fn disable(&mut self) {
        let rcc = unsafe { &(*RCC::ptr()) };
        rcc.cfgr.modify(|_, w| unsafe { w.mcosel().bits(0) });
    }

    pub fn release(self) -> alt::Mco {
        self.pin
    }
}

pub trait MCOExt {
    fn mco(self, src: MCOSrc, psc: Prescaler, rcc: &mut Rcc) -> Mco;
    fn release(self) -> Self;
}

impl<PIN> MCOExt for PIN
where
    PIN: Into<alt::Mco> + TryFrom<alt::Mco>,
{
    fn mco(self, src: MCOSrc, psc: Prescaler, rcc: &mut Rcc) -> Mco {
        let psc_bits = match psc {
            Prescaler::NotDivided => 0b000,
            Prescaler::Div2 => 0b001,
            Prescaler::Div4 => 0b010,
            Prescaler::Div8 => 0b011,
            Prescaler::Div16 => 0b100,
            Prescaler::Div32 => 0b101,
            Prescaler::Div64 => 0b110,
            _ => 0b111,
        };

        rcc.cfgr.modify(|_, w| unsafe { w.mcopre().bits(psc_bits) });

        let src_bits = match src {
            MCOSrc::SysClk => 0b001,
            MCOSrc::HSI => {
                rcc.enable_hsi();
                0b011
            }
            MCOSrc::HSE => {
                rcc.enable_hse(false);
                0b100
            }
            MCOSrc::LSI => {
                rcc.enable_lsi();
                0b110
            }
            MCOSrc::LSE => {
                rcc.enable_lse(false);
                0b111
            }
        };

        Mco {
            src_bits,
            pin: self.into(),
        }
    }

    fn release(self) -> Self {
        self.try_into().unwrap()
    }
}
