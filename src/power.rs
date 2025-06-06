//! Power control

use crate::{
    gpio::*,
    rcc::{Enable, Rcc},
    stm32::PWR,
};

pub enum LowPowerMode {
    StopMode1 = 0b000,
    StopMode2 = 0b001,
    Standby = 0b011,
    Shutdown = 0b111,
}

pub enum PowerMode {
    Run,
    LowPower(LowPowerMode),
    UltraLowPower(LowPowerMode),
}

pub enum WakeUp {
    InternalLine,
    Line1,
    Line2,
    Line3,
    Line4,
    Line6,
}

pub struct Power {
    rb: PWR,
}

impl Power {
    pub fn new(pwr: PWR, rcc: &mut Rcc) -> Self {
        PWR::enable(rcc);
        Self { rb: pwr }
    }

    pub fn get_standby_flag(&mut self) -> bool {
        self.rb.sr1().read().sbf().bit_is_set()
    }

    pub fn get_wakeup_flag<L: Into<WakeUp>>(&self, lane: L) -> bool {
        match lane.into() {
            WakeUp::Line1 => self.rb.sr1().read().wuf1().bit_is_set(),
            WakeUp::Line2 => self.rb.sr1().read().wuf2().bit_is_set(),
            WakeUp::Line3 => self.rb.sr1().read().wuf3().bit_is_set(),
            WakeUp::Line4 => self.rb.sr1().read().wuf4().bit_is_set(),
            WakeUp::Line6 => self.rb.sr1().read().wuf6().bit_is_set(),
            _ => false,
        }
    }

    pub fn clear_wakeup_flag<L: Into<WakeUp>>(&mut self, lane: L) {
        match lane.into() {
            WakeUp::Line1 => _ = self.rb.scr().write(|w| w.cwuf1().set_bit()),
            WakeUp::Line2 => _ = self.rb.scr().write(|w| w.cwuf2().set_bit()),
            WakeUp::Line3 => _ = self.rb.scr().write(|w| w.cwuf3().set_bit()),
            WakeUp::Line4 => _ = self.rb.scr().write(|w| w.cwuf4().set_bit()),
            WakeUp::Line6 => _ = self.rb.scr().write(|w| w.cwuf6().set_bit()),
            _ => {}
        }
    }

    pub fn clear_standby_flag(&mut self) {
        self.rb.scr().write(|w| w.csbf().set_bit());
    }

    pub fn enable_wakeup_lane<L: Into<WakeUp>>(&mut self, lane: L, edge: SignalEdge) {
        assert!(edge != SignalEdge::All);

        let edge = edge == SignalEdge::Falling;
        match lane.into() {
            WakeUp::Line1 => {
                self.rb.cr3().modify(|_, w| w.ewup1().set_bit());
                self.rb.cr4().modify(|_, w| w.wp1().bit(edge));
            }
            WakeUp::Line2 => {
                self.rb.cr3().modify(|_, w| w.ewup2().set_bit());
                self.rb.cr4().modify(|_, w| w.wp2().bit(edge));
            }
            WakeUp::Line3 => {
                self.rb.cr3().modify(|_, w| w.ewup3().set_bit());
                self.rb.cr4().modify(|_, w| w.wp3().bit(edge));
            }
            WakeUp::Line4 => {
                self.rb.cr3().modify(|_, w| w.ewup4().set_bit());
                self.rb.cr4().modify(|_, w| w.wp4().bit(edge));
            }
            WakeUp::Line6 => {
                self.rb.cr3().modify(|_, w| w.ewup6().set_bit());
                self.rb.cr4().modify(|_, w| w.wp6().bit(edge));
            }
            WakeUp::InternalLine => _ = self.rb.cr3().modify(|_, w| w.eiwul().set_bit()),
        }
    }

    pub fn disable_wakeup_lane<L: Into<WakeUp>>(&mut self, lane: L) {
        match lane.into() {
            WakeUp::Line1 => self.rb.cr3().modify(|_, w| w.ewup1().clear_bit()),
            WakeUp::Line2 => self.rb.cr3().modify(|_, w| w.ewup2().clear_bit()),
            WakeUp::Line3 => self.rb.cr3().modify(|_, w| w.ewup3().clear_bit()),
            WakeUp::Line4 => self.rb.cr3().modify(|_, w| w.ewup4().clear_bit()),
            WakeUp::Line6 => self.rb.cr3().modify(|_, w| w.ewup6().clear_bit()),
            WakeUp::InternalLine => self.rb.cr3().modify(|_, w| w.eiwul().clear_bit()),
        };
    }

    pub fn set_mode(&mut self, _mode: PowerMode) {
        todo!();
        // match mode {
        //     PowerMode::Run => {
        //         self.rb.cr1().modify(|_, w| w.lpr().clear_bit());
        //         while !self.rb.sr2().read().reglpf().bit_is_clear() {}
        //     }
        //     PowerMode::LowPower(sm) => {
        //         self.rb.cr3().modify(|_, w| w.ulpen().clear_bit());
        //         self.rb
        //             .cr1
        //             .modify(|_, w| unsafe { w.lpr().set_bit().lpms().bits(sm as u8) });
        //         while !self.rb.sr2().read().reglps().bit_is_set()
        //             || !self.rb.sr2().read().reglpf().bit_is_set()
        //         {}
        //     }
        //     PowerMode::UltraLowPower(sm) => {
        //         self.rb.cr3().modify(|_, w| w.ulpen().set_bit());
        //         self.rb
        //             .cr1
        //             .modify(|_, w| unsafe { w.lpr().set_bit().lpms().bits(sm as u8) });
        //         while !self.rb.sr2().read().reglps().bit_is_set()
        //             || !self.rb.sr2().read().reglpf().bit_is_set()
        //         {}
        //     }
        // }
    }
}

// macro_rules! wakeup_pins {
//     ($($PIN:path: $line:expr,)+) => {
//         $(
//             impl<M> From<&$PIN> for WakeUp {
//                 fn from(_: &$PIN) -> Self {
//                     $line
//                  }
//             }
//         )+
//     }
// }

// wakeup_pins! {
//     Pxx<M>: WakeUp::Line1,
//     Pxx<M>: WakeUp::Line2,
//     Pxx<M>: WakeUp::Line3,
//     Pxx<M>: WakeUp::Line4,
//     Pxx<M>: WakeUp::Line6,
// }

pub trait PowerExt {
    fn constrain(self, rcc: &mut Rcc) -> Power;
}

impl PowerExt for PWR {
    fn constrain(self, rcc: &mut Rcc) -> Power {
        Power::new(self, rcc)
    }
}
