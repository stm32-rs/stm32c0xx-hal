use core::marker::PhantomData;
use core::{fmt, ops::Deref};

use crate::gpio::{
    alt::{SerialAsync as CommonPins, SerialRs485 as Rs485},
    PushPull,
};
use crate::rcc::{self, *};
use crate::serial;
use crate::serial::config::*;
use crate::{gpio, pac, prelude::*};

use nb::block;

/// Serial error
#[derive(Debug)]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity check error
    Parity,
}

/// Interrupt event
pub enum Event {
    /// TXFIFO reaches the threshold
    TXFT = 1 << 27,
    /// This bit is set by hardware when the threshold programmed in RXFTCFG in USART_CR3 register is reached.
    RXFT = 1 << 26,

    /// RXFIFO full
    RXFF = 1 << 24,
    /// TXFIFO empty
    TXFE = 1 << 23,

    /// Active when a communication is ongoing on the RX line
    BUSY = 1 << 16,

    /// Receiver timeout.This bit is set by hardware when the timeout value,
    /// programmed in the RTOR register has lapsed, without any communication.
    RTOF = 1 << 11,
    /// Transmit data register empty. New data can be sent
    Txe = 1 << 7,

    /// Transmission Complete. The last data written in the USART_TDR has been transmitted out of the shift register.
    TC = 1 << 6,
    /// New data has been received
    Rxne = 1 << 5,
    /// Idle line state detected
    Idle = 1 << 4,

    /// Overrun error
    ORE = 1 << 3,

    /// Noise detection flag
    NE = 1 << 2,

    /// Framing error
    FE = 1 << 1,

    /// Parity error
    PE = 1 << 0,
}

impl Event {
    fn val(self) -> u32 {
        self as u32
    }
}

impl Instance for pac::USART1 {
    fn ptr() -> *const pac::usart1::RegisterBlock {
        pac::USART1::ptr()
    }
}
impl Instance for pac::USART2 {
    fn ptr() -> *const pac::usart1::RegisterBlock {
        pac::USART2::ptr()
    }
}

pub trait Instance:
    crate::Sealed
    + rcc::Enable
    + rcc::Reset
    + CommonPins
    + Rs485
    + Deref<Target = pac::usart1::RegisterBlock>
{
    #[doc(hidden)]
    fn ptr() -> *const pac::usart1::RegisterBlock;
}

/// Serial receiver
pub struct Rx<USART: Instance> {
    _usart: PhantomData<USART>,
    _pin: USART::Rx<PushPull>,
}

/// Serial transmitter
pub struct Tx<USART: Instance> {
    _usart: PhantomData<USART>,
    _pin: USART::Tx<PushPull>,
}

/// Serial abstraction
pub struct Serial<USART: Instance> {
    tx: Tx<USART>,
    rx: Rx<USART>,
    usart: USART,
    _depin: Option<USART::De>,
}

/// A filler type for when the Tx pin is unnecessary
pub use gpio::NoPin as NoTx;
/// A filler type for when the Rx pin is unnecessary
pub use gpio::NoPin as NoRx;

pub trait SerialExt: Sized + Instance {
    fn usart(
        self,
        pins: (impl Into<Self::Tx<PushPull>>, impl Into<Self::Rx<PushPull>>),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<Self>, InvalidConfig>;
    fn rs485(
        self,
        pins: (
            impl Into<Self::Tx<PushPull>>,
            impl Into<Self::Rx<PushPull>>,
            impl Into<Self::De>,
        ),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<Self>, InvalidConfig>;
}

impl<USART: Instance> SerialExt for USART {
    fn usart(
        self,
        pins: (impl Into<Self::Tx<PushPull>>, impl Into<Self::Rx<PushPull>>),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<Self>, InvalidConfig> {
        Serial::new(self, pins, config, rcc)
    }
    fn rs485(
        self,
        pins: (
            impl Into<Self::Tx<PushPull>>,
            impl Into<Self::Rx<PushPull>>,
            impl Into<Self::De>,
        ),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<Self>, InvalidConfig> {
        Serial::rs485(self, pins, config, rcc)
    }
}

impl<USART: Instance> fmt::Write for Serial<USART>
where
    Serial<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}

impl<USART: Instance> fmt::Write for Tx<USART>
where
    Tx<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}

impl<USART: Instance> Rx<USART> {
    pub fn listen(&mut self) {
        let usart = unsafe { &(*USART::ptr()) };
        usart.cr1_disabled().modify(|_, w| w.rxneie().set_bit());
    }

    /// Stop listening for an interrupt event
    pub fn unlisten(&mut self) {
        let usart = unsafe { &(*USART::ptr()) };
        usart.cr1_disabled().modify(|_, w| w.rxneie().clear_bit());
    }

    /// Return true if the rx register is not empty (and can be read)
    pub fn is_rxne(&self) -> bool {
        let usart = unsafe { &(*USART::ptr()) };
        usart.isr_disabled().read().rxne().bit_is_set()
    }
}

impl<USART: Instance> hal::serial::Read<u8> for Rx<USART> {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Error> {
        let usart = unsafe { &(*USART::ptr()) };
        let isr = usart.isr_enabled().read();

        Err(if isr.pe().bit_is_set() {
            usart.icr.write(|w| w.pecf().set_bit());
            nb::Error::Other(Error::Parity)
        } else if isr.fe().bit_is_set() {
            usart.icr.write(|w| w.fecf().set_bit());
            nb::Error::Other(Error::Framing)
        } else if isr.ne().bit_is_set() {
            usart.icr.write(|w| w.necf().set_bit());
            nb::Error::Other(Error::Noise)
        } else if isr.ore().bit_is_set() {
            usart.icr.write(|w| w.orecf().set_bit());
            nb::Error::Other(Error::Overrun)
        } else if isr.rxfne().bit_is_set() {
            return Ok(usart.rdr.read().bits() as u8);
        } else {
            nb::Error::WouldBlock
        })
    }
}

impl<USART: Instance> hal::serial::Read<u8> for Serial<USART> {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Error> {
        self.rx.read()
    }
}

impl<USART: Instance> Tx<USART> {
    /// Starts listening for an interrupt event
    pub fn listen(&mut self) {
        let usart = unsafe { &(*USART::ptr()) };
        usart.cr1_disabled().modify(|_, w| w.txeie().set_bit());
    }

    /// Stop listening for an interrupt event
    pub fn unlisten(&mut self) {
        let usart = unsafe { &(*USART::ptr()) };
        usart.cr1_disabled().modify(|_, w| w.txeie().clear_bit());
    }

    /// Return true if the tx register is empty (and can accept data)
    pub fn is_txe(&self) -> bool {
        let usart = unsafe { &(*USART::ptr()) };
        usart.isr_disabled().read().txe().bit_is_set()
    }
}

impl<USART: Instance> hal::serial::Write<u8> for Tx<USART> {
    type Error = Error;

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        let usart = unsafe { &(*USART::ptr()) };
        if usart.isr_disabled().read().tc().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        let usart = unsafe { &(*USART::ptr()) };
        if usart.isr_disabled().read().txe().bit_is_set() {
            usart.tdr.write(|w| unsafe { w.bits(byte as u32) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<USART: Instance> hal::serial::Write<u8> for Serial<USART> {
    type Error = Error;

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.tx.flush()
    }

    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        self.tx.write(byte)
    }
}

impl<USART: Instance> Serial<USART> {
    /// Separates the serial struct into separate channel objects for sending (Tx) and
    /// receiving (Rx)
    pub fn split(self) -> (Tx<USART>, Rx<USART>) {
        (self.tx, self.rx)
    }
}

impl<USART: Instance> Serial<USART> {
    pub fn new(
        usart: USART,
        pins: (
            impl Into<USART::Tx<PushPull>>,
            impl Into<USART::Rx<PushPull>>,
        ),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Self, InvalidConfig> {
        Self::_new(usart, pins, Option::<USART::De>::None, config, rcc)
    }
    fn rs485(
        usart: USART,
        pins: (
            impl Into<USART::Tx<PushPull>>,
            impl Into<USART::Rx<PushPull>>,
            impl Into<USART::De>,
        ),
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Self, InvalidConfig> {
        Self::_new(usart, (pins.0, pins.1), Some(pins.2), config, rcc)
    }
    fn _new(
        usart: USART,
        pins: (
            impl Into<USART::Tx<PushPull>>,
            impl Into<USART::Rx<PushPull>>,
        ),
        depin: Option<impl Into<USART::De>>,
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Self, InvalidConfig> {
        // Enable clock for USART
        USART::enable(rcc);

        let clk = rcc.clocks.apb_clk.raw() as u64;
        let bdr = config.baudrate.0 as u64;
        let clk_mul = 1;
        let div = (clk_mul * clk) / bdr;
        usart.brr.write(|w| unsafe { w.bits(div as u32) });

        // usart.cr1.reset();
        usart.cr2.reset();
        usart.cr3.reset();

        usart.cr2.write(|w| unsafe {
            w.stop().bits(config.stopbits.bits());
            w.swap().bit(config.swap)
        });

        if let Some(timeout) = config.receiver_timeout {
            usart.cr1_enabled().write(|w| w.rtoie().set_bit());
            usart.cr2.modify(|_, w| w.rtoen().set_bit());
            usart.rtor.write(|w| unsafe { w.rto().bits(timeout) });
        }

        usart.cr3.write(|w| unsafe {
            w.txftcfg().bits(config.tx_fifo_threshold.bits());
            w.rxftcfg().bits(config.rx_fifo_threshold.bits());
            w.txftie().bit(config.tx_fifo_interrupt);
            w.rxftie().bit(config.rx_fifo_interrupt)
        });

        usart.cr1_enabled().modify(|_, w| {
            w.ue().set_bit();
            w.te().set_bit();
            w.re().set_bit();
            w.m0().bit(config.wordlength == WordLength::DataBits7);
            w.m1().bit(config.wordlength == WordLength::DataBits9);
            w.pce().bit(config.parity != Parity::ParityNone);
            w.ps().bit(config.parity == Parity::ParityOdd);
            w.fifoen().bit(config.fifo_enable)
        });

        usart.cr3.write(|w| w.dem().bit(depin.is_some()));

        Ok(Serial {
            tx: Tx {
                _usart: PhantomData,
                _pin: pins.0.into(),
            },
            rx: Rx {
                _usart: PhantomData,
                _pin: pins.1.into(),
            },
            usart,
            _depin: depin.map(Into::into),
        })
    }

    /// Starts listening for an interrupt event
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::Rxne => self
                .usart
                .cr1_disabled()
                .modify(|_, w| w.rxneie().set_bit()),
            Event::Txe => self.usart.cr1_disabled().modify(|_, w| w.txeie().set_bit()),
            Event::Idle => self
                .usart
                .cr1_disabled()
                .modify(|_, w| w.idleie().set_bit()),
            _ => {}
        }
    }

    /// Stop listening for an interrupt event
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::Rxne => self
                .usart
                .cr1_disabled()
                .modify(|_, w| w.rxneie().clear_bit()),
            Event::Txe => self
                .usart
                .cr1_disabled()
                .modify(|_, w| w.txeie().clear_bit()),
            Event::Idle => self
                .usart
                .cr1_disabled()
                .modify(|_, w| w.idleie().clear_bit()),
            _ => {}
        }
    }

    /// Check if interrupt event is pending
    pub fn is_pending(&mut self, event: Event) -> bool {
        (self.usart.isr_enabled().read().bits() & event.val()) != 0
    }

    /// Clear pending interrupt
    pub fn unpend(&mut self, event: Event) {
        // mask the allowed bits
        let mask: u32 = 0x123BFF;
        self.usart
            .icr
            .write(|w| unsafe { w.bits(event.val() & mask) });
    }
}

impl<USART: Instance> Tx<USART> {
    /// Returns true if the tx fifo threshold has been reached.
    pub fn fifo_threshold_reached(&self) -> bool {
        let usart = unsafe { &(*USART::ptr()) };
        usart.isr_enabled().read().txft().bit_is_set()
    }
}

impl<USART: Instance> Rx<USART> {
    /// Check if receiver timeout has lapsed
    /// Returns the current state of the ISR RTOF bit
    pub fn timeout_lapsed(&self) -> bool {
        let usart = unsafe { &(*USART::ptr()) };
        usart.isr_enabled().read().rtof().bit_is_set()
    }

    /// Clear pending receiver timeout interrupt
    pub fn clear_timeout(&mut self) {
        let usart = unsafe { &(*USART::ptr()) };
        usart.icr.write(|w| w.rtocf().set_bit());
    }

    /// Returns true if the rx fifo threshold has been reached.
    pub fn fifo_threshold_reached(&self) -> bool {
        let usart = unsafe { &(*USART::ptr()) };
        usart.isr_enabled().read().rxft().bit_is_set()
    }
}
