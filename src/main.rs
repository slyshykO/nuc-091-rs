/*
cargo size --release -- -A -x
cargo objdump --release -- --disassemble --no-show-raw-insn
cargo objcopy --release -- -O binary nuc-091.bin
*/

#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::{entry, exception};
use hal::{i2c::I2c, pac, prelude::*, time::Hertz, timers::*};
use stm32f0xx_hal as hal;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
//use ufmt::uWrite;
use heapless::String;

#[allow(unused)]
use panic_halt as _;

type NucString = String<256>;

// A type definition for the GPIO pin to be used for our LED
type LEDPIN = hal::gpio::gpioa::PA5<hal::gpio::Output<hal::gpio::PushPull>>;

// Mutex protected structure for our shared GPIO pin
static GPIO: Mutex<RefCell<Option<LEDPIN>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut p = pac::Peripherals::take().unwrap();
    // Configure clock to 48 MHz and freeze it
    let mut rcc = p.RCC.configure().sysclk(48.mhz()).freeze(&mut p.FLASH);

    // Configure SysTick timer to trigger every second
    let sys_periph = cortex_m::Peripherals::take().unwrap();
    let mut syst = sys_periph.SYST;

    // configures the system timer to trigger a SysTick exception every second
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(48_000_000 / 1000);
    syst.enable_counter();
    syst.enable_interrupt();

    let gpioc = p.GPIOA.split(&mut rcc);
    cortex_m::interrupt::free(|cs| {
        let led = gpioc.pa5.into_push_pull_output(cs);
        core::mem::swap(&mut Some(led), &mut GPIO.borrow(cs).borrow_mut());
    });

    // Configure pins for I2C
    let gpiob = p.GPIOB.split(&mut rcc);
    let i2c1_pins = cortex_m::interrupt::free(|cs| {
        (
            gpiob.pb8.into_alternate_af1(cs),
            gpiob.pb9.into_alternate_af1(cs),
        )
    });
    let i2c = I2c::i2c1(p.I2C1, i2c1_pins, 100.khz(), &mut rcc);
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let mut timer = if let Err(_e) = display.init() {
        Timer::tim1(p.TIM1, Hertz(1), &mut rcc)
    } else {
        Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();
        Timer::tim1(p.TIM1, Hertz(10), &mut rcc)
    };

    let mut s = NucString::new();
    let mut cnt = 0;
    loop {
        cnt += 1;
        ufmt::uwrite!(&mut s, "{} ", cnt).unwrap();
        Text::new(&s, Point::new(0, 32), text_style)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
        cortex_m::asm::nop();
        nb::block!(timer.wait()).ok();
        s.clear();
        display.clear();
    }
}

#[exception]
unsafe fn SysTick() {
    cortex_m::asm::nop();

    // Our moved LED pin
    static mut LED: Option<LEDPIN> = None;

    // Exception handler state variable
    static mut STATE: u32 = 1;

    // If LED pin was moved into the exception handler, just use it
    if let Some(led) = &mut LED {
        STATE += 1;
        if STATE % 100 == 0 {
            led.toggle().ok();
        }
    }
    // Otherwise move it out of the Mutex protected shared region into our exception handler
    else {
        // Enter critical section
        cortex_m::interrupt::free(|cs| {
            // Swap globally stored data with SysTick private data
            core::mem::swap(&mut LED, &mut GPIO.borrow(cs).borrow_mut());
        });
    }
}
