use super::*;

macro_rules! bus_enable {
    ($PER:ident => $en:ident) => {
        impl Enable for crate::stm32::$PER {
            #[inline(always)]
            fn enable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().set_bit());
            }

            #[inline(always)]
            fn disable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().clear_bit());
            }

            #[inline(always)]
            fn is_enabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::enr(rcc).read().$en().bit_is_set()
            }

            #[inline(always)]
            fn is_disabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::enr(rcc).read().$en().bit_is_clear()
            }

            #[inline(always)]
            unsafe fn enable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::enr(rcc).modify(|_, w| w.$en().set_bit());
            }

            #[inline(always)]
            unsafe fn disable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::enr(rcc).modify(|_, w| w.$en().clear_bit());
            }
        }
    };
}
macro_rules! bus_smenable {
    ($PER:ident => $smen:ident) => {
        impl SMEnable for crate::stm32::$PER {
            #[inline(always)]
            fn sleep_mode_enable(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().set_bit());
            }

            #[inline(always)]
            fn sleep_mode_disable(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().clear_bit());
            }

            #[inline(always)]
            fn is_sleep_mode_enabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::smenr(rcc).read().$smen().bit_is_set()
            }

            #[inline(always)]
            fn is_sleep_mode_disabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::smenr(rcc).read().$smen().bit_is_clear()
            }

            #[inline(always)]
            unsafe fn sleep_mode_enable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().set_bit());
            }

            #[inline(always)]
            unsafe fn sleep_mode_disable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().clear_bit());
            }
        }
    };
}
macro_rules! bus_reset {
    ($PER:ident => $rst:ident) => {
        impl Reset for crate::stm32::$PER {
            #[inline(always)]
            fn reset(rcc: &mut Rcc) {
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().set_bit());
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().clear_bit());
            }
            #[inline(always)]
            unsafe fn reset_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().set_bit());
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().clear_bit());
            }
        }
    };
}

macro_rules! bus {
    ($($PER:ident => ($busX:ty, $($en:ident)?, $($smen:ident)?, $($rst:ident)?),)+) => {
        $(
            impl crate::Sealed for crate::stm32::$PER {}
            impl RccBus for crate::stm32::$PER {
                type Bus = $busX;
            }
            $(bus_enable!($PER => $en);)?
            $(bus_smenable!($PER => $smen);)?
            $(bus_reset!($PER => $rst);)?
        )+
    }
}

bus! {
    CRC => (AHB, crcen, crcsmen, crcrst), // 12
    DMA => (AHB, dma1en, dma1smen, dma1rst), // 0

    DBG => (APB1, dbgen, dbgsmen, dbgrst), // 27
    I2C => (APB1, i2c1en, i2c1smen, i2c1rst), // 21
    PWR => (APB1, pwren, pwrsmen, pwrrst), // 28

    SPI => (APB2, spi1en, spi1smen, spi1rst), // 14
    TIM3 => (APB1, tim3en, tim3smen, tim3rst), // 1
    USART2 => (APB1, usart2en, usart2smen, usart2rst), // 17
    WWDG => (APB1, wwdgen, wwdgsmen,), // 11

    SYSCFG => (APB2, syscfgen, syscfgsmen, syscfgrst), // 0
    ADC => (APB2, adcen, adcsmen, adcrst), // 20
    TIM1 => (APB2, tim1en, tim1smen, tim1rst), // 11
    TIM14 => (APB2, tim14en, tim14smen, tim14rst), // 15
    TIM16 => (APB2, tim16en, tim16smen, tim16rst), // 17
    TIM17 => (APB2, tim17en, tim17smen, tim17rst), // 18
    USART1 => (APB2, usart1en, usart1smen, usart1rst), // 14

    GPIOA => (IOP, gpioaen, gpioasmen, gpioarst), // 0
    GPIOB => (IOP, gpioben, gpiobsmen, gpiobrst), // 1
    GPIOC => (IOP, gpiocen, gpiocsmen, gpiocrst), // 2
    GPIOD => (IOP, gpioden, gpiodsmen, gpiodrst), // 3
    GPIOF => (IOP, gpiofen, gpiofsmen, gpiofrst), // 5
}

#[cfg(feature = "stm32c071")]
bus! {
    I2C2 => (APB1, i2c2en, i2c2smen, i2c2rst), // 21
}
