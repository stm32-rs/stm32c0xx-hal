use core::fmt;
use core::marker::PhantomData;

use crate::gpio::{AltFunction, *};
use crate::prelude::*;
use crate::rcc::*;
use crate::serial;
use crate::serial::config::*;
use crate::stm32::*;

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

/// Serial receiver
pub struct Rx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial transmitter
pub struct Tx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial abstraction
pub struct Serial<USART> {
    tx: Tx<USART>,
    rx: Rx<USART>,
    usart: USART,
}

// Serial TX pin
pub trait TxPin<USART> {
    fn setup(&self);
    fn release(self) -> Self;
}

// Serial RX pin
pub trait RxPin<USART> {
    fn setup(&self);
    fn release(self) -> Self;
}

pub struct NoTx;

impl<USART> TxPin<USART> for NoTx {
    fn setup(&self) {}

    fn release(self) -> Self {
        self
    }
}
pub struct NoRx;

impl<USART> RxPin<USART> for NoRx {
    fn setup(&self) {}

    fn release(self) -> Self {
        self
    }
}

// Driver enable pin
pub trait DriverEnablePin<USART> {
    fn setup(&self);
    fn release(self) -> Self;
}

// Serial pins
pub trait Pins<USART> {
    const DRIVER_ENABLE: bool;

    fn setup(&self);
    fn release(self) -> Self;
}

// Duplex mode
impl<USART, TX, RX> Pins<USART> for (TX, RX)
where
    TX: TxPin<USART>,
    RX: RxPin<USART>,
{
    const DRIVER_ENABLE: bool = false;

    fn setup(&self) {
        self.0.setup();
        self.1.setup();
    }

    fn release(self) -> Self {
        (self.0.release(), self.1.release())
    }
}

// Duplex mode with driver enabled
impl<USART, TX, RX, DE> Pins<USART> for (TX, RX, DE)
where
    TX: TxPin<USART>,
    RX: RxPin<USART>,
    DE: DriverEnablePin<USART>,
{
    const DRIVER_ENABLE: bool = true;

    fn setup(&self) {
        self.0.setup();
        self.1.setup();
        self.2.setup();
    }

    fn release(self) -> Self {
        (self.0.release(), self.1.release(), self.2.release())
    }
}

pub trait SerialExt<USART> {
    fn usart<PINS: Pins<USART>>(
        self,
        pins: PINS,
        config: serial::Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<USART>, InvalidConfig>;
}

impl<USART> fmt::Write for Serial<USART>
where
    Serial<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s
            .as_bytes()
            .iter()
            .map(|c| block!(self.write(*c)))
            .next_back();
        Ok(())ß
    }
}

impl<USART> fmt::Write for Tx<USART>
where
    Tx<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s
            .as_bytes()
            .iter()
            .map(|c| block!(self.write(*c)))
            .next_back();
        Ok(())
    }
}

macro_rules! uart_shared {
    ($USARTX:ident, $dmamux_rx:ident, $dmamux_tx:ident,
        tx: [ $(($PTX:ident, $TAF:expr),)+ ],
        rx: [ $(($PRX:ident, $RAF:expr),)+ ],
        de: [ $(($PDE:ident, $DAF:expr),)+ ]) => {

        $(
            impl<MODE> TxPin<$USARTX> for $PTX<MODE> {
                fn setup(&self) {
                    self.set_alt_mode($TAF)
                }

                fn release(self) -> Self {
                    self
                }
            }
        )+

        $(
            impl<MODE> RxPin<$USARTX> for $PRX<MODE> {
                fn setup(&self) {
                    self.set_alt_mode($RAF)
                }

                fn release(self) -> Self {
                    self
                }
            }
        )+

        $(
            impl<MODE> DriverEnablePin<$USARTX> for $PDE<MODE> {
                fn setup(&self) {
                    self.set_alt_mode($DAF)
                }

                fn release(self) -> Self {
                    self
                }
            }
        )+

        impl Rx<$USARTX> {
            pub fn listen(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1().modify(|_, w| w.rxneie().bit(true));
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1().modify(|_, w| w.rxneie().clear_bit());
            }

            /// Return true if the rx register is not empty (and can be read)
            pub fn is_rxne(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr().read().rxfne().bit_is_set()
            }
        }

        impl hal::serial::Read<u8> for Rx<$USARTX> {
            type Error = Error;

            fn read(&mut self) -> nb::Result<u8, Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                let isr = usart.isr().read();

                Err(
                    if isr.pe().bit_is_set() {
                        usart.icr().write(|w| w.pecf().bit(true));
                        nb::Error::Other(Error::Parity)
                    } else if isr.fe().bit_is_set() {
                        usart.icr().write(|w| w.fecf().bit(true));
                        nb::Error::Other(Error::Framing)
                    } else if isr.ne().bit_is_set() {
                        usart.icr().write(|w| w.necf().bit(true));
                        nb::Error::Other(Error::Noise)
                    } else if isr.ore().bit_is_set() {
                        usart.icr().write(|w| w.orecf().bit(true));
                        nb::Error::Other(Error::Overrun)
                    } else if isr.rxfne().bit_is_set() {
                        return Ok(usart.rdr().read().bits() as u8)
                    } else {
                        nb::Error::WouldBlock
                    }
                )
            }
        }

        impl hal::serial::Read<u8> for Serial<$USARTX> {
            type Error = Error;

            fn read(&mut self) -> nb::Result<u8, Error> {
                self.rx.read()
            }
        }

        impl Tx<$USARTX> {
            /// Starts listening for an interrupt event
            pub fn listen(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1().modify(|_, w| w.txfeie().bit(true));
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1().modify(|_, w| w.txfeie().clear_bit());
            }

            /// Return true if the tx register is empty (and can accept data)
            pub fn is_txfe(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr().read().txfe().bit_is_set()
            }
        }

        impl hal::serial::Write<u8> for Tx<$USARTX> {
            type Error = Error;

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                if usart.isr().read().tc().bit_is_set() {
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }

            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                if usart.isr().read().txfe().bit_is_set() {
                    usart.tdr().write(|w| unsafe { w.bits(byte as u32) });
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }
        }

        impl hal::serial::Write<u8> for Serial<$USARTX> {
            type Error = Error;

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                self.tx.flush()
            }

            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                self.tx.write(byte)
            }
        }

        impl Serial<$USARTX> {

            /// Separates the serial struct into separate channel objects for sending (Tx) and
            /// receiving (Rx)
            pub fn split(self) -> (Tx<$USARTX>, Rx<$USARTX>) {
                (self.tx, self.rx)
            }

        }
    }
}

macro_rules! uart {
    ($USARTX:ident,
        $usartX:ident, $clk_mul:expr
    ) => {
        impl SerialExt<$USARTX> for $USARTX {
            fn usart<PINS: Pins<$USARTX>>(
                self,
                pins: PINS,
                config: serial::Config,
                rcc: &mut Rcc,
            ) -> Result<Serial<$USARTX>, InvalidConfig> {
                Serial::$usartX(self, pins, config, rcc)
            }
        }

        impl Serial<$USARTX> {
            pub fn $usartX<PINS: Pins<$USARTX>>(
                usart: $USARTX,
                pins: PINS,
                config: serial::Config,
                rcc: &mut Rcc,
            ) -> Result<Self, InvalidConfig> {
                // Enable clock for USART
                $USARTX::enable(rcc);

                let clk = rcc.clocks.apb_clk.raw() as u64;
                let bdr = config.baudrate.0 as u64;
                let clk_mul = 1;
                let div = (clk_mul * clk) / bdr;
                usart.brr().write(|w| unsafe { w.bits(div as u32) });

                // usart.cr1.reset();
                usart.cr2().reset();
                usart.cr3().reset();

                usart.cr2().write(|w| unsafe {
                    w.stop()
                        .bits(config.stopbits.bits())
                        .swap()
                        .bit(config.swap)
                });

                if let Some(timeout) = config.receiver_timeout {
                    usart.cr1().write(|w| w.rtoie().bit(true));
                    usart.cr2().modify(|_, w| w.rtoen().bit(true));
                    usart.rtor().write(|w| unsafe { w.rto().bits(timeout) });
                }

                usart.cr3().write(|w| unsafe {
                    w.txftcfg()
                        .bits(config.tx_fifo_threshold.bits())
                        .rxftcfg()
                        .bits(config.rx_fifo_threshold.bits())
                        .txftie()
                        .bit(config.tx_fifo_interrupt)
                        .rxftie()
                        .bit(config.rx_fifo_interrupt)
                });

                usart.cr1().modify(|_, w| {
                    w.ue()
                        .bit(true)
                        .te()
                        .bit(true)
                        .re()
                        .bit(true)
                        .m0()
                        .bit(config.wordlength == WordLength::DataBits7)
                        .m1()
                        .bit(config.wordlength == WordLength::DataBits9)
                        .pce()
                        .bit(config.parity != Parity::ParityNone)
                        .ps()
                        .bit(config.parity == Parity::ParityOdd)
                        .fifoen()
                        .bit(config.fifo_enable)
                });

                usart.cr3().write(|w| w.dem().bit(PINS::DRIVER_ENABLE));

                // Enable pins
                pins.setup();

                Ok(Serial {
                    tx: Tx {
                        _usart: PhantomData,
                    },
                    rx: Rx {
                        _usart: PhantomData,
                    },
                    usart,
                })
            }

            /// Starts listening for an interrupt event
            pub fn listen(&mut self, event: Event) {
                match event {
                    Event::Rxne => _ = self.usart.cr1().modify(|_, w| w.rxneie().bit(true)),
                    Event::TXFE => _ = self.usart.cr1().modify(|_, w| w.txeie().bit(true)),
                    Event::Idle => _ = self.usart.cr1().modify(|_, w| w.idleie().bit(true)),
                    _ => {}
                }
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self, event: Event) {
                match event {
                    Event::Rxne => _ = self.usart.cr1().modify(|_, w| w.rxneie().clear_bit()),
                    Event::TXFE => _ = self.usart.cr1().modify(|_, w| w.txeie().clear_bit()),
                    Event::Idle => _ = self.usart.cr1().modify(|_, w| w.idleie().clear_bit()),
                    _ => {}
                }
            }

            /// Check if interrupt event is pending
            pub fn is_pending(&mut self, event: Event) -> bool {
                (self.usart.isr().read().bits() & event.val()) != 0
            }

            /// Clear pending interrupt
            pub fn unpend(&mut self, event: Event) {
                // mask the allowed bits
                let mask: u32 = 0x123BFF;
                self.usart
                    .icr()
                    .write(|w| unsafe { w.bits(event.val() & mask) });
            }
        }

        impl Tx<$USARTX> {
            /// Returns true if the tx fifo threshold has been reached.
            pub fn fifo_threshold_reached(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr().read().txft().bit_is_set()
            }
        }

        impl Rx<$USARTX> {
            /// Check if receiver timeout has lapsed
            /// Returns the current state of the ISR RTOF bit
            pub fn timeout_lapsed(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr().read().rtof().bit_is_set()
            }

            /// Clear pending receiver timeout interrupt
            pub fn clear_timeout(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.icr().write(|w| w.rtocf().bit(true));
            }

            /// Returns true if the rx fifo threshold has been reached.
            pub fn fifo_threshold_reached(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr().read().rxft().bit_is_set()
            }
        }
    };
}

uart_shared!(USART1, USART1_RX, USART1_TX,
    tx: [
        (PA0, AltFunction::AF4),
        (PA9, AltFunction::AF1),
        (PB6, AltFunction::AF0),
        (PC14, AltFunction::AF0),
    ],
    rx: [
        (PA1, AltFunction::AF4),
        (PA8, AltFunction::AF14),
        (PA10, AltFunction::AF1),
        (PB2, AltFunction::AF0),
        (PB7, AltFunction::AF0),
    ],
    de: [
        (PA12, AltFunction::AF1),
        (PA14, AltFunction::AF12),
        (PA15, AltFunction::AF4),
        (PB3, AltFunction::AF4),
        (PB6, AltFunction::AF4),
    ]
);

uart_shared!(USART2, USART2_RX, USART2_TX,
    tx: [
        (PA2, AltFunction::AF1),
        (PA4, AltFunction::AF1),
        (PA8, AltFunction::AF1),
        (PA14, AltFunction::AF1),
    ],
    rx: [
        (PA3, AltFunction::AF1),
        (PA5, AltFunction::AF1),
        (PA13, AltFunction::AF4),
        (PA14, AltFunction::AF9),
        (PA15, AltFunction::AF1),
    ],
    de: [
        (PA1, AltFunction::AF1),
        (PB9, AltFunction::AF1),
        (PC14, AltFunction::AF9),
    ]
);

uart!(USART1, usart1, 1);
uart!(USART2, usart2, 1);
