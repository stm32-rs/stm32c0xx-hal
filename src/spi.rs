use crate::gpio;
use crate::pac::spi as spi1;
use crate::rcc::{self, Rcc};
use crate::time::Hertz;
use core::ops::Deref;
use core::ptr;
pub use hal::spi::{Mode, Phase, Polarity, MODE_0, MODE_1, MODE_2, MODE_3};

/// SPI error
#[derive(Debug)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
}

/// A filler type for when the SCK pin is unnecessary
pub use gpio::NoPin as NoSck;
/// A filler type for when the Miso pin is unnecessary
pub use gpio::NoPin as NoMiso;
/// A filler type for when the Mosi pin is unnecessary
pub use gpio::NoPin as NoMosi;

// Implemented by all SPI instances
pub trait Instance:
    crate::Sealed
    + Deref<Target = spi1::RegisterBlock>
    + rcc::Enable
    + rcc::Reset
    + gpio::alt::SpiCommon
{
    #[doc(hidden)]
    fn ptr() -> *const spi1::RegisterBlock;
}

#[derive(Debug)]
pub struct Spi<SPI: Instance> {
    spi: SPI,
    pins: (SPI::Sck, SPI::Miso, SPI::Mosi),
}

pub trait SpiExt: Sized + Instance {
    fn spi(
        self,
        pins: (
            impl Into<Self::Sck>,
            impl Into<Self::Miso>,
            impl Into<Self::Mosi>,
        ),
        mode: Mode,
        freq: Hertz,
        rcc: &mut Rcc,
    ) -> Spi<Self>;
}

impl<SPI: Instance> SpiExt for SPI {
    fn spi(
        self,
        pins: (
            impl Into<Self::Sck>,
            impl Into<Self::Miso>,
            impl Into<Self::Mosi>,
        ),
        mode: Mode,
        freq: Hertz,
        rcc: &mut Rcc,
    ) -> Spi<Self> {
        Spi::new(self, pins, mode, freq, rcc)
    }
}

impl<SPI: Instance> Spi<SPI> {
    pub fn new(
        spi: SPI,
        pins: (
            impl Into<SPI::Sck>,
            impl Into<SPI::Miso>,
            impl Into<SPI::Mosi>,
        ),
        mode: Mode,
        speed: Hertz,
        rcc: &mut Rcc,
    ) -> Self {
        SPI::enable(rcc);
        SPI::reset(rcc);

        // disable SS output
        spi.cr2.write(|w| w.ssoe().clear_bit());

        let br = match rcc.clocks.apb_clk / speed {
            0 => unreachable!(),
            1..=2 => 0b000,
            3..=5 => 0b001,
            6..=11 => 0b010,
            12..=23 => 0b011,
            24..=47 => 0b100,
            48..=95 => 0b101,
            96..=191 => 0b110,
            _ => 0b111,
        };

        spi.cr2
            .write(|w| unsafe { w.frxth().set_bit().ds().bits(0b111).ssoe().clear_bit() });

        // Enable pins
        let pins = (pins.0.into(), pins.1.into(), pins.2.into());

        spi.cr1.write(|w| unsafe {
            w.cpha().bit(mode.phase == Phase::CaptureOnSecondTransition);
            w.cpol().bit(mode.polarity == Polarity::IdleHigh);
            w.mstr().set_bit();
            w.br().bits(br);
            w.lsbfirst().clear_bit();
            w.ssm().set_bit();
            w.ssi().set_bit();
            w.rxonly().clear_bit();
            w.bidimode().clear_bit();
            w.ssi().set_bit();
            w.spe().set_bit()
        });

        Spi { spi, pins }
    }

    pub fn data_size(&mut self, nr_bits: u8) {
        self.spi
            .cr2
            .modify(|_, w| unsafe { w.ds().bits(nr_bits - 1) });
    }

    pub fn half_duplex_enable(&mut self, enable: bool) {
        self.spi.cr1.modify(|_, w| w.bidimode().bit(enable));
    }

    pub fn half_duplex_output_enable(&mut self, enable: bool) {
        self.spi.cr1.modify(|_, w| w.bidioe().bit(enable));
    }

    pub fn release(self) -> (SPI, (SPI::Sck, SPI::Miso, SPI::Mosi)) {
        (self.spi, self.pins)
    }
}

impl<SPI: Instance> hal::spi::FullDuplex<u8> for Spi<SPI> {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Error> {
        let sr = self.spi.sr.read();

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            nb::Error::Other(Error::ModeFault)
        } else if sr.crcerr().bit_is_set() {
            nb::Error::Other(Error::Crc)
        } else if sr.rxne().bit_is_set() {
            // NOTE(read_volatile) read only 1 byte (the svd2rust API only allows
            // reading a half-word)
            return Ok(unsafe { ptr::read_volatile(&self.spi.dr as *const _ as *const u8) });
        } else {
            nb::Error::WouldBlock
        })
    }

    fn send(&mut self, byte: u8) -> nb::Result<(), Error> {
        let sr = self.spi.sr.read();

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            nb::Error::Other(Error::ModeFault)
        } else if sr.crcerr().bit_is_set() {
            nb::Error::Other(Error::Crc)
        } else if sr.txe().bit_is_set() {
            // NOTE(write_volatile) see note above
            unsafe { ptr::write_volatile(&self.spi.dr as *const _ as *mut u8, byte) }
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }
}

impl<SPI: Instance> ::hal::blocking::spi::transfer::Default<u8> for Spi<SPI> {}

impl<SPI: Instance> ::hal::blocking::spi::write::Default<u8> for Spi<SPI> {}
