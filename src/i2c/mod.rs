#[cfg(feature = "i2c-blocking")]
pub mod blocking;

#[cfg(feature = "i2c-nonblocking")]
pub mod nonblocking;

use core::ops::Deref;

#[cfg(feature = "i2c-nonblocking")]
pub use nonblocking::*;

pub mod config;

use crate::gpio;
use crate::pac::{self, i2c as i2c1};
use crate::rcc::*;
pub use config::Config;

#[derive(Debug, Clone, Copy)]
pub enum SlaveAddressMask {
    MaskNone = 0,
    MaskOneBit,
    MaskTwoBits,
    MaskThreeBits,
    MaskFourBits,
    MaskFiveBits,
    MaskSixBits,
    MaskAllBits,
}

#[derive(Debug, Clone, Copy)]
pub enum I2cResult<'a> {
    Data(u16, I2cDirection, &'a [u8]), // contains address, direction and data slice reference
    Addressed(u16, I2cDirection),      // a slave is addressed by a master
}

#[derive(Debug, Clone, Copy)]
pub enum I2cDirection {
    MasterReadSlaveWrite = 0,
    MasterWriteSlaveRead = 1,
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    AddressMatch,
    Rxne,
}

/// I2C error
#[derive(Debug, Clone, Copy)]
pub enum Error {
    Overrun,
    Nack,
    PECError,
    BusError,
    ArbitrationLost,
    IncorrectFrameSize(usize),
}

pub trait Instance:
    crate::Sealed + Deref<Target = i2c1::RegisterBlock> + Enable + Reset + gpio::alt::I2cCommon
{
    #[doc(hidden)]
    fn ptr() -> *const i2c1::RegisterBlock;
}

// Implemented by all I2C instances
macro_rules! i2c {
    ($I2C:ty: $I2c:ident) => {
        pub type $I2c = I2c<$I2C>;

        impl Instance for $I2C {
            fn ptr() -> *const i2c1::RegisterBlock {
                <$I2C>::ptr() as *const _
            }
        }
    };
}

i2c! { pac::I2C: I2c1 }

pub trait I2cExt: Sized + Instance {
    fn i2c<SDA, SCL>(
        self,
        pins: (impl Into<Self::Scl>, impl Into<Self::Sda>),
        config: impl Into<Config>,
        rcc: &mut Rcc,
    ) -> I2c<Self>;
}

/// I2C abstraction
#[cfg(feature = "i2c-blocking")]
pub struct I2c<I2C: Instance> {
    i2c: I2C,
    pins: (I2C::Scl, I2C::Sda),
}

#[cfg(feature = "i2c-nonblocking")]
pub struct I2c<I2C: Instance> {
    i2c: I2C,
    pins: (I2C::Scl, I2C::Sda),
    address: u16,
    watchdog: u16, // on each start set to 10, on each stop set to 0
    index: usize,
    length: usize,
    errors: usize,            // global error counter, reset on read
    length_write_read: usize, // for a master write_read operation this remembers the size of the read operation
    // for a slave device this must be 0
    data: [u8; 255], // during transfer the driver will be the owner of the buffer
}
