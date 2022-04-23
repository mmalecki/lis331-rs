use esp_idf_sys as _;

use esp_idf_hal::i2c;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;

use lis331::{Lis331, SlaveAddr};
use lis331::accelerometer::RawAccelerometer;

use std::thread;
use std::time::Duration;

fn main() {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    println!("Starting LIS331 hello-world");

    let config = <i2c::config::MasterConfig as Default>::default().baudrate(100.kHz().into());
    let mut i2c = i2c::Master::<i2c::I2C0, _, _>::new(i2c, i2c::MasterPins { sda, scl }, config).unwrap();

    let mut sensor = Lis331::new_i2c(i2c, SlaveAddr::Default).unwrap();

    loop {
        let accel = sensor.accel_raw().unwrap();
        println!("{:?}", accel);
        thread::sleep(Duration::from_millis(250));
    }
}
