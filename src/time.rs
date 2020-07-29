use core::fmt::Debug;

#[cfg(feature = "chrono")]
use chrono::{self, Datelike, Local, TimeZone, Timelike};

/// A DOS compatible date.
///
/// Used by `DirEntry` time-related methods.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Date {
    /// Full year - [1980, 2107]
    pub year: u16,
    /// Month of the year - [1, 12]
    pub month: u16,
    /// Day of the month - [1, 31]
    pub day: u16,
}

impl Date {
    pub(crate) fn decode(dos_date: u16) -> Self {
        let (year, month, day) = ((dos_date >> 9) + 1980, (dos_date >> 5) & 0xF, dos_date & 0x1F);
        Self { year, month, day }
    }

    pub(crate) fn encode(self) -> u16 {
        ((self.year - 1980) << 9) | (self.month << 5) | self.day
    }
}

/// A DOS compatible time.
///
/// Used by `DirEntry` time-related methods.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Time {
    /// Hours after midnight - [0, 23]
    pub hour: u16,
    /// Minutes after the hour - [0, 59]
    pub min: u16,
    /// Seconds after the minute - [0, 59]
    pub sec: u16,
    /// Milliseconds after the second - [0, 999]
    pub millis: u16,
}

impl Time {
    pub(crate) fn decode(dos_time: u16, dos_time_hi_res: u8) -> Self {
        let hour = dos_time >> 11;
        let min = (dos_time >> 5) & 0x3F;
        let sec = (dos_time & 0x1F) * 2 + u16::from(dos_time_hi_res / 100);
        let millis = u16::from(dos_time_hi_res % 100) * 10;
        Self { hour, min, sec, millis }
    }

    pub(crate) fn encode(self) -> (u16, u8) {
        let dos_time = (self.hour << 11) | (self.min << 5) | (self.sec / 2);
        // safe cast: value in range [0, 199]
        let dos_time_hi_res = ((self.millis / 10) + (self.sec % 2) * 100) as u8;
        (dos_time, dos_time_hi_res)
    }
}

/// A DOS compatible date and time.
///
/// Used by `DirEntry` time-related methods.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct DateTime {
    /// A date part
    pub date: Date,
    // A time part
    pub time: Time,
}

impl DateTime {
    pub(crate) fn decode(dos_date: u16, dos_time: u16, dos_time_hi_res: u8) -> Self {
        Self {
            date: Date::decode(dos_date),
            time: Time::decode(dos_time, dos_time_hi_res),
        }
    }
}

#[cfg(feature = "chrono")]
impl From<Date> for chrono::Date<Local> {
    fn from(date: Date) -> Self {
        Local.ymd(i32::from(date.year), u32::from(date.month), u32::from(date.day))
    }
}

#[cfg(feature = "chrono")]
impl From<DateTime> for chrono::DateTime<Local> {
    fn from(date_time: DateTime) -> Self {
        chrono::Date::<Local>::from(date_time.date).and_hms_milli(
            u32::from(date_time.time.hour),
            u32::from(date_time.time.min),
            u32::from(date_time.time.sec),
            u32::from(date_time.time.millis),
        )
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::Date<Local>> for Date {
    fn from(date: chrono::Date<Local>) -> Self {
        Self {
            year: date.year() as u16,   // safe cast unless it is after year 65,536
            month: date.month() as u16, // safe cast: value in range [1, 12]
            day: date.day() as u16,     // safe cast: value in range [1, 31]
        }
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<Local>> for DateTime {
    fn from(date_time: chrono::DateTime<Local>) -> Self {
        let millis = date_time.nanosecond() / 1_000_000;
        Self {
            date: Date::from(date_time.date()),
            time: Time {
                hour: date_time.hour() as u16,  // safe cast: value in range [0, 23]
                min: date_time.minute() as u16, // safe cast: value in range [0, 59]
                sec: date_time.second() as u16, // safe cast: value in range [0, 59]
                millis: millis as u16,          // safe cast: value in range [0, 999]
            },
        }
    }
}

/// A current time and date provider.
///
/// Provides a custom implementation for a time resolution used when updating directory entry time fields.
/// `TimeProvider` is specified by the `time_provider` property in `FsOptions` struct.
pub trait TimeProvider: Debug {
    fn get_current_date(&self) -> Date;
    fn get_current_date_time(&self) -> DateTime;
}

/// `TimeProvider` implementation that returns current local time retrieved from `chrono` crate.
#[cfg(feature = "chrono")]
#[derive(Debug, Clone, Copy, Default)]
pub struct ChronoTimeProvider {
    _dummy: (),
}

#[cfg(feature = "chrono")]
impl ChronoTimeProvider {
    pub fn new() -> Self {
        Self { _dummy: () }
    }
}

#[cfg(feature = "chrono")]
impl TimeProvider for ChronoTimeProvider {
    fn get_current_date(&self) -> Date {
        Date::from(chrono::Local::now().date())
    }

    fn get_current_date_time(&self) -> DateTime {
        DateTime::from(chrono::Local::now())
    }
}

/// `TimeProvider` implementation that always returns DOS minimal date-time (1980-01-01 00:00:00).
#[derive(Debug, Clone, Copy, Default)]
pub struct NullTimeProvider {
    _dummy: (),
}

impl NullTimeProvider {
    pub fn new() -> Self {
        Self { _dummy: () }
    }
}

impl TimeProvider for NullTimeProvider {
    fn get_current_date(&self) -> Date {
        Date::decode(0)
    }

    fn get_current_date_time(&self) -> DateTime {
        DateTime::decode(0, 0, 0)
    }
}

/// Default time provider implementation.
///
/// Defined as `ChronoTimeProvider` if `chrono` feature is enabled. Otherwise defined as `NullTimeProvider`.
#[cfg(feature = "chrono")]
pub type DefaultTimeProvider = ChronoTimeProvider;
#[cfg(not(feature = "chrono"))]
pub type DefaultTimeProvider = NullTimeProvider;

#[cfg(test)]
mod tests {
    use super::{Date, Time};

    #[test]
    fn date_encode_decode() {
        let d = Date {
            year: 2055,
            month: 7,
            day: 23,
        };
        let x = d.encode();
        assert_eq!(x, 38647);
        assert_eq!(d, Date::decode(x));
    }

    #[test]
    fn time_encode_decode() {
        let t1 = Time {
            hour: 15,
            min: 3,
            sec: 29,
            millis: 990,
        };
        let t2 = Time { sec: 18, ..t1 };
        let t3 = Time { millis: 40, ..t1 };
        let (x1, y1) = t1.encode();
        let (x2, y2) = t2.encode();
        let (x3, y3) = t3.encode();
        assert_eq!((x1, y1), (30830, 199));
        assert_eq!((x2, y2), (30825, 99));
        assert_eq!((x3, y3), (30830, 104));
        assert_eq!(t1, Time::decode(x1, y1));
        assert_eq!(t2, Time::decode(x2, y2));
        assert_eq!(t3, Time::decode(x3, y3));
    }
}
