//! Sharing data between a main thread and an interrupt handler safely using `unsafe`
//! code blocks in contexts where they can't cause
//! (Undefined Behavior)[https://doc.rust-lang.org/reference/behavior-considered-undefined.html].
//!
//! This example uses the normally `unsafe`
//! [`Peripherals::steal()`][steal] method to safely share access to msp430 peripherals between a
//! main thread and interrupt. All uses of [`steal()`] are commented to explain _why_ its usage
//! is safe in that particular context.
//!
//! As with [timer] and [timer-oncecell], this example uses the `TIMER0_A1` interrupt to blink
//! LEDs on the [MSP-EXP430G2](http://www.ti.com/tool/MSP-EXP430G2) development kit.
//!
//! [steal]: msp430g2553::Peripherals::steal
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use msp430::interrupt as mspint;
use msp430_rt::entry;
use msp430g2553::{interrupt, Peripherals};

#[entry]
fn main(cs: CriticalSection) -> ! {
    // Safe because interrupts are disabled after a reset.
    let p = unsafe { Peripherals::steal() };

    let wdt = &p.WATCHDOG_TIMER;
    wdt.wdtctl.write(|w| {
        unsafe { w.bits(0x5A00) } // password
        .wdthold().set_bit()
    });

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1dir.modify(|_, w| w.p0().set_bit()
                                  .p6().set_bit());
    port_1_2.p1out.modify(|_, w| w.p0().set_bit()
                                  .p6().clear_bit());

    let clock = &p.SYSTEM_CLOCK;
    clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2());
    clock.bcsctl1.modify(|_, w| w.diva().diva_1());

    let timer = &p.TIMER0_A3;
    timer.ta0ccr0.write(|w| unsafe { w.bits(1200) });
    timer.ta0ctl.modify(|_, w| w.tassel().tassel_1()
                                .mc().mc_1());
    timer.ta0cctl1.modify(|_, w| w.ccie().set_bit());
    timer.ta0ccr1.write(|w| unsafe { w.bits(600) });

    mspint::enable_cs(cs);

    loop {
        mspint::free(|_cs| {
            // Do something while interrupts are disabled.
        })
    }
}

#[interrupt]
fn TIMER0_A1(_cs: CriticalSection) {
    // Safe because msp430 disables interrupts on handler entry. Therefore the handler
    // has full control/access to peripherals without data races.
    let p = unsafe { Peripherals::steal() };

    let timer = &p.TIMER0_A3;
    timer.ta0cctl1.modify(|_, w| w.ccifg().clear_bit());

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1out.modify(|r, w| w.p0().bit(!r.p0().bit())
                                  .p6().bit(!r.p6().bit()));
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
