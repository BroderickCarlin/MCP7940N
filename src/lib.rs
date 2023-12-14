#![no_std]

use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use embedded_hal::i2c::I2c;

pub enum ClockSource {
    ExtCrystal,
    ExtClock,
}

pub struct ClockConfig {
    pub enabled: bool,
    pub clock_source: ClockSource,
}

pub struct Mcp7940n<I> {
    i2c: I,
}

impl<I> Mcp7940n<I> {
    const ADDRESS: u8 = 0b110_1111;

    pub fn new(i2c: I) -> Self {
        Self { i2c }
    }

    pub fn destroy(self) -> I {
        self.i2c
    }
}

impl<I: I2c> Mcp7940n<I> {
    pub fn configure_clock(&mut self, config: &ClockConfig) -> Result<(), I::Error> {
        let mut data = [0u8; 9];
        // Just read all the data - bit excessive since we only need 2 of these registers but lets us make sure to
        // keep all the data synced with a single write
        self.i2c
            .write_read(Self::ADDRESS, &[0x00], &mut data[1..])?;

        if config.enabled {
            data[1] |= 0b1000_0000;
        } else {
            data[1] &= 0b0111_1111;
        }

        match config.clock_source {
            ClockSource::ExtClock => data[8] |= 0b0000_1000,
            ClockSource::ExtCrystal => data[8] &= 0b1111_0111,
        }

        self.i2c.write(Self::ADDRESS, &data)
    }

    pub fn osc_running(&mut self) -> Result<bool, I::Error> {
        let mut data = [0u8; 1];

        self.i2c.write_read(Self::ADDRESS, &[0x03], &mut data)?;

        Ok(data[0] & 0b0010_0000 != 0)
    }

    pub fn now(&mut self) -> Result<NaiveDateTime, I::Error> {
        let mut data = [0u8; 7];
        self.i2c.write_read(Self::ADDRESS, &[0x00], &mut data)?;

        let sec_ten = (data[0] & 0b0111_0000) >> 4;
        let sec_ones = data[0] & 0b0000_1111;

        let secs = (sec_ten * 10) + sec_ones;

        let min_ten = (data[1] & 0b0111_0000) >> 4;
        let min_ones = data[1] & 0b0000_1111;

        let min = (min_ten * 10) + min_ones;

        let hr_12 = (data[2] & 0b0100_0000) != 0;
        let hr_ones = data[2] & 0b0000_1111;

        // We want to always convert to 24hr time
        let hour = if hr_12 {
            let pm = (data[2] & 0b0010_0000) != 0;
            let hr_ten = (data[2] & 0b0001_0000) >> 4;
            let hr = (hr_ten * 10) + hr_ones;

            if pm && hr != 12 {
                hr + 12
            } else if !pm && hr == 12 {
                0
            } else {
                hr
            }
        } else {
            let hr_ten = (data[2] & 0b0011_0000) >> 4;
            (hr_ten * 10) + hr_ones
        };

        let day_ten = (data[4] & 0b0011_0000) >> 4;
        let day_ones = data[4] & 0b0000_1111;

        let day = (day_ten * 10) + day_ones;

        let month_ten = (data[5] & 0b0001_0000) >> 4;
        let month_ones = data[5] & 0b0000_1111;

        let month = (month_ten * 10) + month_ones;

        let year_ten = (data[6] & 0b1111_0000) >> 4;
        let year_ones = data[6] & 0b0000_1111;

        let year = (year_ten * 10) as i32 + year_ones as i32 + 2000;

        let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap();
        Ok(date
            .and_hms_opt(hour as u32, min as u32, secs as u32)
            .unwrap())
    }

    pub fn set_datetime(&mut self, now: &NaiveDateTime) -> Result<(), I::Error> {
        let time = now.time();
        let date = now.date();

        let seconds_tens = (time.second() / 10) as u8;
        let seconds_ones = (time.second() % 10) as u8;

        let minutes_tens = (time.minute() / 10) as u8;
        let minutes_ones = (time.minute() % 10) as u8;

        let hours_tens = (time.hour() / 10) as u8;
        let hours_ones = (time.hour() % 10) as u8;

        let day_tens = (date.day() / 10) as u8;
        let day_ones = (date.day() % 10) as u8;

        let month_tens = (date.month() / 10) as u8;
        let month_ones = (date.month() % 10) as u8;

        let year_tens = ((date.year() - 2000) / 10) as u8;
        let year_ones = ((date.year() - 2000) % 10) as u8;

        let mut data = [0u8; 8];
        self.i2c
            .write_read(Self::ADDRESS, &[0x00], &mut data[1..])?;

        data[1] &= 0b1000_0000;
        data[1] |= seconds_tens << 4;
        data[1] |= seconds_ones;

        data[2] &= 0b1000_0000;
        data[2] |= minutes_tens << 4;
        data[2] |= minutes_ones;

        data[3] &= 0b1000_0000;
        data[3] |= hours_tens << 4;
        data[3] |= hours_ones;

        data[5] &= 0b1100_0000;
        data[5] |= day_tens << 4;
        data[5] |= day_ones;

        data[6] &= 0b1100_0000;
        data[6] |= month_tens << 4;
        data[6] |= month_ones;

        data[7] = 0;
        data[7] |= year_tens << 4;
        data[7] |= year_ones;

        self.i2c.write(Self::ADDRESS, &data)
    }
}
