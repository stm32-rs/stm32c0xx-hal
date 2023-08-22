//! I2C
use crate::i2c::config::Config;
use crate::i2c::{self, Error, I2c, I2cDirection, I2cExt, Instance};
use crate::rcc::*;
use hal::blocking::i2c::{Read, Write, WriteRead};

pub trait I2cSlave {
    /// Enable/Disable Slave Byte Control. Default SBC is switched on.
    /// For master write/read the transaction should start with sbc disabled.
    /// So ACK will be send on the last received byte.
    /// Before the send phase SBC should be enabled again.
    fn slave_sbc(&mut self, sbc_enabled: bool);

    /// An optional tuple is returned with the address as sent by the master. The address is for 7 bit in range of 0..127
    fn slave_addressed(&mut self) -> Result<Option<(u16, I2cDirection)>, Error>;

    /// Wait until this slave is addressed by the master.
    /// A tuple is returned with the address as sent by the master. The address is for 7 bit in range of 0..127
    fn slave_wait_addressed(&mut self) -> Result<(u16, I2cDirection), Error>;

    /// Start reading the bytes, send by the master . If OK returned, all bytes are transferred
    /// If the master want to send more bytes than the slave can recieve the slave will NACK the n+1 byte
    /// In this case the function will return IncorrectFrameSize(bytes.len() + 1)
    /// If the master did send a STOP before all bytes are recieve, the slave will return IncorrectFrameSize(actual nr of bytes send)
    fn slave_read(&mut self, bytes: &mut [u8]) -> Result<(), Error>;

    /// Start writing the bytes, the master want to receive. If OK returned, all bytes are transferred
    /// If the master wants more data than bytes.len()  the master will run into a timeout, This function will return Ok(())
    /// If the master wants less data than bytes.len(), the function will return  IncorrectFrameSize(bytes.len() + 1)
    fn slave_write(&mut self, bytes: &[u8]) -> Result<(), Error>;
}

/// Sequence to flush the TXDR register. This resets the TXIS and TXE flags
macro_rules! flush_txdr {
    ($i2c:expr) => {
        // If a pending TXIS flag is set, write dummy data to TXDR
        if $i2c.isr.read().txis().bit_is_set() {
            $i2c.txdr.write(|w| unsafe { w.txdata().bits(0) });
        }

        // If TXDR is not flagged as empty, write 1 to flush it
        if $i2c.isr.read().txe().bit_is_set() {
            $i2c.isr.write(|w| w.txe().set_bit());
        }
    };
}
/// Sequence to flush the RXDR register. This resets the TXIS and TXE flags
macro_rules! flush_rxdr {
    ($i2c:expr) => {
        if $i2c.isr.read().rxne().bit_is_set() {
            // flush
            let _ = $i2c.rxdr.read().rxdata().bits();
        };
    };
}

/// Check the isr flags, with 2 types of exit
/// In case of hard errors the error will be returned, also forcing the caller of this function to return
/// In all other case the macro will return without a result
macro_rules! busy_wait {
    ($i2c:expr, $flag:ident, $variant:ident, $idx:ident, $buflen:ident) => {
        loop {
            let isr = $i2c.isr.read();

            if isr.$flag().$variant() {
                break
            } else  if isr.berr().bit_is_set() {
                $i2c.icr.write(|w| w.berrcf().set_bit());
                return Err(Error::BusError);
            } else if isr.arlo().bit_is_set() {
                $i2c.icr.write(|w| w.arlocf().set_bit());
                return Err(Error::ArbitrationLost);
            } else if isr.nackf().bit_is_set() {
                $i2c.icr.write(|w| w.nackcf().set_bit());
                // Make one extra loop to wait on the stop condition
            } else if isr.tcr().bit_is_set() {
                // This condition Will only happen when reload == 1 and sbr == 1 (slave) and nbytes was written.
                // Send a NACK, set nbytes to clear tcr flag
                $i2c.cr2.modify(|_, w| unsafe {
                    w.nack().set_bit().nbytes().bits(1 as u8)
                });
                // Make one extra loop here to wait on the stop condition
            } else if isr.addr().bit_is_set() {
                // in case of a master write_read operation, this flag is the only exit for the function.
                // Leave the bit set, so it can be detected in the wait_addressed function
                if $idx == $buflen {
                    return Ok( () )
                } else {
                  return Err(Error::IncorrectFrameSize($idx))
                }
            } else if isr.stopf().bit_is_set() {
                flush_txdr!($i2c);
                // Clear the stop condition flag
                $i2c.icr.write(|w| w.stopcf().set_bit());
                if $idx == $buflen {
                    return Ok( () )
                } else
                if $idx == 0 {
                    return Err(Error::Nack)
                } else
                {
                  return Err(Error::IncorrectFrameSize($idx))
                }
            } else  {
                // try again
            }
        }
    };
}

impl<I2C: Instance> I2cExt for I2C {
    fn i2c(
        self,
        pins: (impl Into<Self::Scl>, impl Into<Self::Sda>),
        config: impl Into<Config>,
        rcc: &mut Rcc,
    ) -> I2c<Self> {
        I2c::new(self, pins, config, rcc)
    }
}

impl<I2C: Instance> I2c<I2C> {
    pub fn new(
        i2c: I2C,
        pins: (impl Into<I2C::Scl>, impl Into<I2C::Sda>),
        config: impl Into<Config>,
        rcc: &mut Rcc,
    ) -> Self {
        let config = config.into();
        I2C::enable(rcc);
        I2C::reset(rcc);

        // Make sure the I2C unit is disabled so we can configure it
        i2c.cr1.modify(|_, w| w.pe().clear_bit());

        // Setup protocol timings
        let timing_bits = config.timing_bits(rcc.clocks.apb_clk);
        i2c.timingr.write(|w| unsafe { w.bits(timing_bits) });

        // Enable the I2C processing
        i2c.cr1.modify(|_, w| unsafe {
            w.pe().set_bit();
            w.dnf().bits(config.digital_filter);
            w.anfoff().bit(!config.analog_filter)
        });

        if config.slave_address_1 > 0 {
            i2c.oar1.write(|w| unsafe {
                w.oa1().bits(config.slave_address_1);
                w.oa1mode().bit(config.address_11bits);
                w.oa1en().set_bit()
            });
            // Enable acknowlidge control
            i2c.cr1.modify(|_, w| w.sbc().set_bit());
        }

        if config.slave_address_2 > 0 {
            i2c.oar2.write(|w| unsafe {
                w.oa2msk().bits(config.slave_address_mask as u8);
                w.oa2().bits(config.slave_address_2);
                w.oa2en().set_bit()
            });
            // Enable acknowlidge control
            i2c.cr1.modify(|_, w| w.sbc().set_bit());
        }

        // Enable pins
        let pins = (pins.0.into(), pins.1.into());

        I2c { i2c, pins }
    }

    pub fn listen(&mut self, ev: i2c::Event) {
        match ev {
            i2c::Event::AddressMatch => self.i2c.cr1.modify(|_, w| w.addrie().set_bit()),
            i2c::Event::Rxne => self.i2c.cr1.modify(|_, w| w.rxie().set_bit()),
        }
    }

    pub fn unlisten(&mut self, ev: i2c::Event) {
        match ev {
            i2c::Event::AddressMatch => self.i2c.cr1.modify(|_, w| w.addrie().clear_bit()),
            i2c::Event::Rxne => self.i2c.cr1.modify(|_, w| w.rxie().clear_bit()),
        }
    }

    pub fn clear_irq(&mut self, ev: i2c::Event) {
        match ev {
            i2c::Event::AddressMatch => self.i2c.icr.write(|w| w.addrcf().set_bit()),
            _ => {}
        }
    }

    pub fn release(self) -> (I2C, (I2C::Scl, I2C::Sda)) {
        (self.i2c, self.pins)
    }
}

impl<I2C: Instance> WriteRead for I2c<I2C> {
    type Error = Error;

    fn write_read(
        &mut self,
        addr: u8,
        snd_buffer: &[u8],
        rcv_buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        let sndlen = snd_buffer.len();
        let rcvlen = rcv_buffer.len();
        assert!(sndlen < 256 && sndlen > 0);
        assert!(rcvlen < 256 && rcvlen > 0);

        // Wait for any previous address sequence to end automatically.
        // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
        while self.i2c.cr2.read().start().bit_is_set() {}

        // flush i2c tx register
        self.i2c.isr.write(|w| w.txe().set_bit());

        // Set START and prepare to send `bytes`.
        // The START bit can be set even if the bus is BUSY or
        // I2C is in slave mode.
        self.i2c.cr2.write(|w| unsafe {
            // Set number of bytes to transfer
            w.nbytes().bits(sndlen as u8);
            // Set address to transfer to/from
            w.sadd().bits((addr << 1) as u16);
            // 7-bit addressing mode
            w.add10().clear_bit();
            // Set transfer direction to write
            w.rd_wrn().clear_bit();
            // Software end mode
            w.autoend().clear_bit();
            w.reload().clear_bit();
            // Start transfer
            w.start().set_bit()
        });
        let mut idx = 0;
        // Wait until we are allowed to send data
        // (START has been ACKed or last byte went through)
        // macro will return false when the tc bit is set
        for byte in snd_buffer {
            busy_wait!(self.i2c, txis, bit_is_set, idx, sndlen);
            // Put byte on the wire
            self.i2c.txdr.write(|w| unsafe { w.txdata().bits(*byte) });
            idx += 1;
        }
        // Wait until the write finishes before beginning to read.
        let dummy = 0xFE;
        busy_wait!(self.i2c, tc, bit_is_set, idx, dummy);

        // reSTART and prepare to receive bytes into `rcv_buffer`
        self.i2c.cr2.write(|w| unsafe {
            // Set number of bytes to transfer
            w.nbytes().bits(rcvlen as u8);
            // Set address to transfer to/from
            w.sadd().bits((addr << 1) as u16);
            // 7-bit addressing mode
            w.add10().clear_bit();
            // Set transfer direction to read
            w.rd_wrn().set_bit();
            // Automatic end mode
            w.autoend().set_bit();
            w.reload().clear_bit();
            // Start transfer
            w.start().set_bit()
        });

        idx = 0;
        loop {
            // Wait until we have received something. Handle all state in busy_wait macro
            busy_wait!(self.i2c, rxne, bit_is_set, idx, rcvlen);
            if idx < rcvlen {
                rcv_buffer[idx] = self.i2c.rxdr.read().rxdata().bits();
                idx += 1;
            }
        }
    }
}

impl<I2C: Instance> Write for I2c<I2C> {
    type Error = Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let buflen = bytes.len();
        assert!(buflen < 256 && buflen > 0);

        // Wait for any previous address sequence to end automatically.
        // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
        while self.i2c.cr2.read().start().bit_is_set() {}

        self.i2c.cr2.modify(|_, w| unsafe {
            // Start transfer
            w.start().set_bit();
            // Set number of bytes to transfer
            w.nbytes().bits(buflen as u8);
            // Set address to transfer to/from
            w.sadd().bits((addr << 1) as u16);
            // Set transfer direction to write
            w.rd_wrn().clear_bit();
            // Automatic end mode
            w.autoend().set_bit();
            w.reload().clear_bit()
        });

        let mut idx = 0;
        loop {
            // Wait until we are allowed to send data, handle all state in busy_wait macro
            busy_wait!(self.i2c, txis, bit_is_set, idx, buflen);

            // Put byte on the wire
            if idx < buflen {
                self.i2c
                    .txdr
                    .write(|w| unsafe { w.txdata().bits(bytes[idx]) });
                idx += 1;
            }
        }
    }
}

impl<I2C: Instance> Read for I2c<I2C> {
    type Error = Error;

    fn read(&mut self, addr: u8, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let buflen = bytes.len();
        assert!(buflen < 256 && buflen > 0);

        // Wait for any previous address sequence to end automatically.
        // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
        while self.i2c.cr2.read().start().bit_is_set() {}
        // Flush rxdr register
        let _ = self.i2c.rxdr.read().rxdata().bits();

        // Set START and prepare to receive bytes into `buffer`.
        // The START bit can be set even if the bus
        // is BUSY or I2C is in slave mode.
        self.i2c.cr2.modify(|_, w| unsafe {
            // Start transfer
            w.start().set_bit();
            // Set number of bytes to transfer
            w.nbytes().bits(buflen as u8);
            // Set address to transfer to/from
            w.sadd().bits((addr << 1) as u16);
            // Set transfer direction to read
            w.rd_wrn().set_bit();
            // automatic end mode
            w.autoend().set_bit();
            w.reload().clear_bit()
        });
        let mut idx = 0;
        loop {
            // Wait until we have received something
            busy_wait!(self.i2c, rxne, bit_is_set, idx, buflen);
            if idx < buflen {
                bytes[idx] = self.i2c.rxdr.read().rxdata().bits();
                idx += 1;
            }
        }
    }
}

impl<I2C: Instance> I2cSlave for I2c<I2C> {
    fn slave_sbc(&mut self, sbc_enabled: bool) {
        // Enable Slave byte control
        self.i2c.cr1.modify(|_, w| w.sbc().bit(sbc_enabled));
    }

    fn slave_addressed(&mut self) -> Result<Option<(u16, I2cDirection)>, Error> {
        if self.i2c.isr.read().addr().bit_is_set() {
            let isr = self.i2c.isr.read();
            let current_address = isr.addcode().bits() as u16;

            // if the dir bit is set it is a master write slave read operation
            let direction = if isr.dir().bit_is_set() {
                I2cDirection::MasterReadSlaveWrite
            } else {
                I2cDirection::MasterWriteSlaveRead
            };
            // do not yet release the clock stretching here.
            // In the slave read function the nbytes is send, for this the addr bit must be set
            Ok(Some((current_address, direction)))
        } else {
            Ok(None)
        }
    }

    fn slave_wait_addressed(&mut self) -> Result<(u16, I2cDirection), Error> {
        loop {
            if let Some(res) = self.slave_addressed()? {
                return Ok(res);
            }
        }
    }

    fn slave_write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let buflen = bytes.len();
        assert!(buflen < 256 && buflen > 0);

        // Set the nbytes and prepare to send bytes into `buffer`.
        self.i2c
            .cr2
            .modify(|_, w| unsafe { w.nbytes().bits(buflen as u8).reload().clear_bit() });
        // flush i2c tx register
        self.i2c.isr.write(|w| w.txe().set_bit());
        // end address phase, release clock stretching
        self.i2c.icr.write(|w| w.addrcf().set_bit());

        let mut idx = 0;
        loop {
            // wait until we are allowed to send the byte. Handle all state in macro
            busy_wait!(self.i2c, txis, bit_is_set, idx, buflen);

            // Put byte on the wire
            if idx < buflen {
                self.i2c
                    .txdr
                    .write(|w| unsafe { w.txdata().bits(bytes[idx]) });
                idx += 1;
            } else {
                // we will never reach here. In case the master wants to read more than buflen
                // the hardware will send 0xFF
                // Also means that on slave side we cannot detect this error case
                self.i2c.txdr.write(|w| unsafe { w.txdata().bits(0x21) });
            }
        }
    }

    fn slave_read(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        let buflen = bytes.len();
        assert!(buflen < 256 && buflen > 0);

        // Set the nbytes START and prepare to receive bytes into `buffer`.
        self.i2c.cr2.modify(|_, w| unsafe {
            // Set number of bytes to transfer: maximum as all incoming bytes will be ACK'ed
            w.nbytes().bits(buflen as u8);
            // during sending nbytes automatically send a ACK, stretch clock after last byte
            w.reload().set_bit()
        });
        // end address phase, release clock stretching
        self.i2c.icr.write(|w| w.addrcf().set_bit());
        flush_rxdr!(self.i2c);

        let mut idx = 0;
        loop {
            // Wait until we have received something.
            busy_wait!(self.i2c, rxne, bit_is_set, idx, buflen);

            // read byte from wire
            if idx < buflen {
                bytes[idx] = self.i2c.rxdr.read().rxdata().bits();
                idx += 1;
            }
        }
    }
}
