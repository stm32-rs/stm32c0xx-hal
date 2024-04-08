//! External interrupt controller
use crate::gpio::SignalEdge;
use crate::stm32::EXTI;

/// EXTI trigger event
#[derive(Eq, PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum Event {
    GPIO0 = 0,
    GPIO1 = 1,
    GPIO2 = 2,
    GPIO3 = 3,
    GPIO4 = 4,
    GPIO5 = 5,
    GPIO6 = 6,
    GPIO7 = 7,
    GPIO8 = 8,
    GPIO9 = 9,
    GPIO10 = 10,
    GPIO11 = 11,
    GPIO12 = 12,
    GPIO13 = 13,
    GPIO14 = 14,
    GPIO15 = 15,
    RTC = 19,
    I2C1 = 23,
    USART1 = 25,
    LSE_CSS = 31,
}

impl Event {
    pub(crate) fn from_code(n: u8) -> Event {
        match n {
            0 => Event::GPIO0,
            1 => Event::GPIO1,
            2 => Event::GPIO2,
            3 => Event::GPIO3,
            4 => Event::GPIO4,
            5 => Event::GPIO5,
            6 => Event::GPIO6,
            7 => Event::GPIO7,
            8 => Event::GPIO8,
            9 => Event::GPIO9,
            10 => Event::GPIO10,
            11 => Event::GPIO11,
            12 => Event::GPIO12,
            13 => Event::GPIO13,
            14 => Event::GPIO14,
            15 => Event::GPIO15,
            _ => unreachable!(),
        }
    }
}

const TRIGGER_MAX: u8 = 15;

pub trait ExtiExt {
    fn wakeup(&self, ev: Event);
    fn listen(&self, ev: Event, edge: SignalEdge);
    fn unlisten(&self, ev: Event);
    fn is_pending(&self, ev: Event, edge: SignalEdge) -> bool;
    fn unpend(&self, ev: Event);
}

impl ExtiExt for EXTI {
    fn listen(&self, ev: Event, edge: SignalEdge) {
        let line = ev as u8;
        assert!(line <= TRIGGER_MAX);
        let mask = 1 << line;
        match edge {
            SignalEdge::Rising => {
                self.rtsr1().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
            SignalEdge::Falling => {
                self.ftsr1().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
            SignalEdge::All => {
                self.rtsr1().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
                self.ftsr1().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
        }
        self.wakeup(ev);
    }

    fn wakeup(&self, ev: Event) {
        self.imr1()
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << ev as u8) });
    }

    fn unlisten(&self, ev: Event) {
        self.unpend(ev);

        let line = ev as u8;
        let mask = !(1 << line);
        self.imr1().modify(|r, w| unsafe { w.bits(r.bits() & mask) });
        if line <= TRIGGER_MAX {
            self.rtsr1().modify(|r, w| unsafe { w.bits(r.bits() & mask) });
            self.ftsr1().modify(|r, w| unsafe { w.bits(r.bits() & mask) });
        }
    }

    fn is_pending(&self, ev: Event, edge: SignalEdge) -> bool {
        let line = ev as u8;
        if line > TRIGGER_MAX {
            return false;
        }
        let mask = 1 << line;
        match edge {
            SignalEdge::Rising => self.rpr1().read().bits() & mask != 0,
            SignalEdge::Falling => self.fpr1().read().bits() & mask != 0,
            SignalEdge::All => {
                (self.rpr1().read().bits() & mask != 0) && (self.fpr1().read().bits() & mask != 0)
            }
        }
    }

    fn unpend(&self, ev: Event) {
        let line = ev as u8;
        if line <= TRIGGER_MAX {
            self.rpr1().modify(|_, w| unsafe { w.bits(1 << line) });
            self.fpr1().modify(|_, w| unsafe { w.bits(1 << line) });
        }
    }
}
