//! Power control

use cortex_m::asm::wfe;
use crate::{
    gpio::*,
    rcc::{Enable, Rcc},
    stm32::PWR,
};
#[derive(PartialEq)]
pub enum PowerMode {
    Run,
    Sleep,
    Stop,
    Standby,
    Shutdown,
}

pub enum WakeUp {
    InternalLine,
    Line1,
    Line2,
    Line3,
    Line4,
    Line6,
}

pub enum GpioPort {
    A,
    B,
    C,
    D,
    F,
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
        let edge = match edge {
            SignalEdge::Rising => false,
            SignalEdge::Falling => true,
            SignalEdge::All => return,
        };

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

    pub fn set_mode(&mut self, mode: PowerMode) {
        let lpms_value = match mode {
            PowerMode::Stop => 0b000,
            PowerMode::Standby => 0b011,
            PowerMode::Shutdown => 0b100,
            _ => return,
        };
        self.rb.cr1().modify(|_, w| unsafe {w.lpms().bits(lpms_value)});
        wfe(); // Stimulus. Can be wfi() too
    }

    pub fn flash_memory_state_low_power(&mut self, mode: PowerMode, powerdown: bool) {
        if mode == PowerMode::Sleep {
            self.rb.cr1().modify(|_, w| w.fpd_slp().bit(powerdown));
        } else if mode == PowerMode::Stop {
            self.rb.cr1().modify(|_, w| w.fpd_stop().bit(powerdown));
        }
    }

    #[cfg(feature = "stm32c071")]
    pub fn read_backup_registers(&mut self, reg_no: u8) -> Option<u16> {
        match reg_no {
            0 => Some(self.rb.bkp0r().read().bkp().bits()),
            1 => Some(self.rb.bkp1r().read().bkp().bits()),
            2 => Some(self.rb.bkp2r().read().bkp().bits()),
            3 => Some(self.rb.bkp3r().read().bkp().bits()),
            _ => None,
        }
    }

    #[cfg(feature = "stm32c071")]
    pub fn write_backup_registers(&mut self, reg_no: u8, value: u16) {
        match reg_no {
            0 => {
                self.rb.bkp0r().write(|w| unsafe {w.bkp().bits(value)});
            }
            1 => {
                self.rb.bkp1r().write(|w| unsafe {w.bkp().bits(value)});
            }
            2 => {
                self.rb.bkp2r().write(|w| unsafe {w.bkp().bits(value)});
            }
            3 => {
                self.rb.bkp3r().write(|w| unsafe {w.bkp().bits(value)});
            }
            _ => return,
        }
    }
}

/// Set the pull-up/pull-down state of GPIO pins during any power mode
impl Power {
    pub fn set_pull_up_down_state(&mut self, state: bool) {
        self.rb.cr3().modify(|_, w| w.apc().bit(state));
    }
    fn modify_pin(bits: u32, pin: u8, state: bool) -> u32 {
        if state {
            bits | (1 << pin)
        } else {
            bits & !(1 << pin)
        }
    }

    pub fn set_pull_down(&mut self, port: GpioPort, pin: u8, state: bool) {
        match port {
            GpioPort::A => {
                if pin < 16 {
                    self.rb.pdcra().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::B => {
                if pin < 16 {
                    self.rb.pdcrb().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::C => {
                #[cfg(feature = "stm32c011")]
                if pin == 14 || pin == 15 {
                    self.rb.pdcrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c031")]
                if (6..=7).contains(&pin) || (13..=15).contains(&pin) {
                    self.rb.pdcrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(not(any(feature = "stm32c011", feature = "stm32c031")))]
                if pin < 16 {
                    self.rb.pdcrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::D => {
                #[cfg(feature = "stm32c031")]
                if (0..=3).contains(&pin) {
                    self.rb.pdcrd().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c071")]
                if (0..=3).contains(&pin) || (8..=9).contains(&pin) {
                    self.rb.pdcrd().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::F => {
                #[cfg(feature = "stm32c011")]
                if pin == 2 {
                    self.rb.pdcrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c031")]
                if (0..=2).contains(&pin) {
                    self.rb.pdcrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c071")]
                if (0..=3).contains(&pin) {
                    self.rb.pdcrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
        }
    }

    pub fn set_pull_up(&mut self, port: GpioPort, pin: u8, state: bool) {
        match port {
            GpioPort::A => {
                if pin < 16 {
                    self.rb.pucra().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::B => {
                #[cfg(feature = "stm32c011")]
                if pin == 6 || pin == 7 {
                    self.rb.pucrc().modify(|r, w| { // note: pucrc ici à confirmer
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(not(feature = "stm32c011"))]
                if pin < 16 {
                    self.rb.pucrb().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::C => {
                #[cfg(feature = "stm32c011")]
                if pin == 14 || pin == 15 {
                    self.rb.pucrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c031")]
                if (6..=7).contains(&pin) || (13..=15).contains(&pin) {
                    self.rb.pucrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(not(any(feature = "stm32c011", feature = "stm32c031")))]
                if pin < 16 {
                    self.rb.pucrc().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::D => {
                #[cfg(feature = "stm32c031")]
                if (0..=3).contains(&pin) {
                    self.rb.pucrd().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c071")]
                if (0..=3).contains(&pin) || (8..=9).contains(&pin) {
                    self.rb.pucrd().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
            GpioPort::F => {
                #[cfg(feature = "stm32c011")]
                if pin == 2 {
                    self.rb.pucrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c031")]
                if (0..=2).contains(&pin) {
                    self.rb.pucrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }

                #[cfg(feature = "stm32c071")]
                if (0..=3).contains(&pin) {
                    self.rb.pucrf().modify(|r, w| {
                        unsafe { w.bits(Self::modify_pin(r.bits(), pin, state)) }
                    });
                }
            }
        }
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
