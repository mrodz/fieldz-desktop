pub mod algorithm;

use std::fmt::Display;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, TimeDelta, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;
use thiserror::Error;

#[derive(Debug, Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct AvailabilityWindow {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl Display for AvailabilityWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn d<T: TimeZone>(f: &mut std::fmt::Formatter<'_>, date: DateTime<T>) -> std::fmt::Result {
            write!(
                f,
                "{:02}:{:02} {}/{}/{}",
                date.hour(),
                date.minute(),
                date.day(),
                date.month(),
                date.year()
            )
        }

        d(f, self.start)?;
        write!(f, " to ")?;
        d(f, self.end)
    }
}

impl AvailabilityWindow {
    pub fn single_day(
        day: u32,
        month: u32,
        year: u32,
        start: (u32, u32),
        end: (u32, u32),
    ) -> Result<Self> {
        let start = Utc
            .with_ymd_and_hms(year.try_into()?, month, day, start.0, start.1, 0)
            .earliest()
            .context("ambiguous start date")?;
        let end = Utc
            .with_ymd_and_hms(year.try_into()?, month, day, end.0, end.1, 0)
            .earliest()
            .context("ambiguous end date")?;

        if end < start {
            bail!("End time ({end:?}) is before ({start:?})");
        }

        Ok(Self { start, end })
    }

    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self> {
        if end < start {
            bail!("End time ({end:?}) is before ({start:?})");
        }

        Ok(Self { start, end })
    }

    pub fn contains(&self, time: DateTime<Utc>) -> bool {
        self.start < time && time < self.end
    }

    pub fn add_error(&mut self, err: TimeDelta) -> Result<()> {
        self.start = self
            .start
            .checked_sub_signed(err)
            .context("could not prepare lower err")?;
        self.end = self
            .end
            .checked_add_signed(err)
            .context("could not prepare upper err")?;
        Ok(())
    }

    #[inline(always)]
    pub fn overlap_fast(lhs: &Self, rhs: &Self) -> bool {
        lhs.start <= rhs.end && lhs.end >= rhs.start
    }

    pub fn overlap(mut lhs: Self, mut rhs: Self, err: TimeDelta) -> Result<bool> {
        lhs.add_error(err)?;
        rhs.add_error(err)?;

        Ok(Self::overlap_fast(&lhs, &rhs))
    }

    pub(crate) fn as_lossy_window(&self) -> Result<LossyAvailability, LossyAvailabilityError> {
        LossyAvailability::new(&self.start, &self.end)
    }
}

#[cfg(test)]
mod compressed_availability_window {
    use crate::window;

    #[test]
    fn normal_normal() {
        let time = window!(10/2/2023 from 8:00 to 9:00).unwrap();

        let compressed = time.as_lossy_window().unwrap();

        let reverted = compressed.as_availability_window_lossy();

        assert_eq!(time, reverted);
    }

    #[test]
    fn normal_half() {
        let time = window!(10/2/2023 from 8:00 to 9:30).unwrap();

        let compressed = time.as_lossy_window().unwrap();

        let reverted = compressed.as_availability_window_lossy();

        assert_eq!(time, reverted);
    }

    #[test]
    fn half_normal() {
        let time = window!(10/2/2023 from 8:30 to 9:00).unwrap();

        let compressed = time.as_lossy_window().unwrap();

        let reverted = compressed.as_availability_window_lossy();

        assert_eq!(time, reverted);
    }

    #[test]
    fn half_half() {
        let time = window!(10/2/2023 from 8:30 to 9:30).unwrap();

        let compressed = time.as_lossy_window().unwrap();

        let reverted = compressed.as_availability_window_lossy();

        assert_eq!(time, reverted);
    }

    #[test]
    fn lost_data() {
        // 8:15 will round up to 8:30
        let time = window!(10/2/2023 from 8:15 to 9:30).unwrap();

        let compressed = time.as_lossy_window().unwrap();

        let reverted = compressed.as_availability_window_lossy();

        assert_eq!(window!(10/2/2023 from 8:30 to 9:30).unwrap(), reverted);
    }
}

/// # Lossy compression for a date range.
///
/// Some caveats: compression cannot represent a date range longer than ~3 years.
/// * "8:00" -> "8:00"
/// * "8:01" -> "8:30"
/// * "8:59" -> "8:30"
/// * "9:00" -> "9:00"
///
/// ## Memory layout
///
/// ```txt
/// ================================ (32 bits)
///               ^|              ^|
/// ---end-hours--^|-start-hours--^|
///                |               |
///      end 1/2 hr?   start 1/2 hr?   
///
/// ```
///
/// ## This means that we can save a lot of space on date ranges.
/// From:
/// size = 24 (0x18), align = 0x4
/// To:
/// size = 4, align = 0x4

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LossyAvailability(u32);

#[derive(Error, Debug)]
pub enum LossyAvailabilityError {
    #[error("This compression algorithm requires that the delta of the hours of a date and a constant point fit inside 15 bits, but got {0}")]
    Overflow(i64),
}

impl LossyAvailability {
    /// This function does not validate the safety of the input and whether the
    /// range satisfies the precondition for compression. It will not panic either,
    /// so passing invalid parameters will result in undefined behavior.
    ///
    /// # Safety
    /// * `start_cutoff_seconds` must be the [Unix timestamp](https://en.wikipedia.org/wiki/Unix_time)
    ///   of the lowest date the compressed range can contain. It must also match the date
    ///   used in the decompressor, else you will recieve garbage.
    /// * `start` and `end` must be within three years (roughly) of `start_cutoff_seconds`,
    ///   otherwise you will observe wrapping and undefined behavior.
    pub const unsafe fn new_unchecked(
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        start_cutoff_seconds: i64,
    ) -> Self {
        let start_milliseconds: i64 = start.timestamp() - start_cutoff_seconds;
        let mut start_hours: u32 = (start_milliseconds / SECONDS_TO_HOURS) as u32;
        let mut rem = start_milliseconds % SECONDS_TO_HOURS;

        start_hours <<= 1;

        if rem != 0 {
            start_hours |= 1 << 0;
        }

        let end_milliseconds: i64 = end.timestamp() - start_cutoff_seconds;
        let mut end_hours: u32 = (end_milliseconds / SECONDS_TO_HOURS) as u32;
        rem = end_milliseconds % SECONDS_TO_HOURS;

        end_hours <<= 17;

        if rem != 0 {
            end_hours |= 1 << 16;
        }

        Self(start_hours | end_hours)
    }

    /// Create a new compressed date range given a start and end date.
    pub fn new(start: &DateTime<Utc>, end: &DateTime<Utc>) -> Result<Self, LossyAvailabilityError> {
        let start_milliseconds: i64 = start.timestamp() - *Y_2023;

        // the compiler will optimize this away
        let start_hours = start_milliseconds / SECONDS_TO_HOURS;
        let mut rem = start_milliseconds % SECONDS_TO_HOURS;

        let Ok(mut start_hours) = <i64 as TryInto<u32>>::try_into(start_hours) else {
            return Err(LossyAvailabilityError::Overflow(start_hours));
        };

        start_hours <<= 1;

        if start_hours & 0xffff0001 != 0 {
            return Err(LossyAvailabilityError::Overflow(start_hours as i64));
        }

        if rem != 0 {
            start_hours |= 1;
        }

        let end_milliseconds: i64 = end.timestamp() - *Y_2023;

        // the compiler will also optimize this away
        let end_hours = end_milliseconds / SECONDS_TO_HOURS;
        rem = end_milliseconds % SECONDS_TO_HOURS;

        let Ok(mut end_hours) = <i64 as TryInto<u32>>::try_into(end_hours) else {
            return Err(LossyAvailabilityError::Overflow(end_hours));
        };

        end_hours <<= 17;

        if end_hours & 0x1ffff != 0 {
            panic!("too big! {end_hours:#034b}")
        }

        if rem != 0 {
            end_hours |= 1 << 16;
        }

        Ok(Self(start_hours | end_hours))
    }

    fn as_datetime_lossy(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        const SECONDS_IN_HALF_AN_HOUR: i64 = 1800;

        let mut start = self.start_data();
        let mut end = self.end_data();

        let start_has_half_hour = start & 1 == 1;
        start >>= 1;
        let mut start_seconds = start as i64 * SECONDS_TO_HOURS;

        if start_has_half_hour {
            start_seconds += SECONDS_IN_HALF_AN_HOUR;
        }

        let end_has_half_hour = end & 1 == 1;
        end >>= 1;
        let mut end_seconds = end as i64 * SECONDS_TO_HOURS;

        if end_has_half_hour {
            end_seconds += SECONDS_IN_HALF_AN_HOUR;
        }

        let start_hours = DateTime::from_timestamp(start_seconds + *Y_2023, 0).unwrap();
        let end_hours = DateTime::from_timestamp(end_seconds + *Y_2023, 0).unwrap();

        (start_hours, end_hours)
    }

    /// This function will decompress the time window.
    pub fn as_availability_window_lossy(&self) -> AvailabilityWindow {
        let (start, end) = self.as_datetime_lossy();
        AvailabilityWindow::new(start, end).unwrap()
    }

    pub const fn start_data(&self) -> u16 {
        (self.0 & 0x0000ffff) as u16
    }

    pub const fn end_data(&self) -> u16 {
        ((self.0 & 0xffff0000) >> 16) as u16
    }

    #[inline(always)]
    pub const fn overlap_fast(lhs: &Self, rhs: &Self) -> bool {
        lhs.start_data() <= rhs.end_data() && lhs.start_data() >= rhs.end_data()
    }
}

const SECONDS_TO_HOURS: i64 = 3_600;

lazy_static! {
    static ref Y_2023: i64 = Utc
        .with_ymd_and_hms(2006, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp();
}

trait TeamLike {
    fn id(&self) -> u8;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Team {
    name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Field {
    name: Option<String>,
    time_slots: Vec<AvailabilityWindow>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ScheduledInput {
    teams: Vec<Team>,
    fields: Vec<Field>,
}

impl ScheduledInput {
    pub fn new(teams: impl AsRef<[Team]>, fields: impl AsRef<[Field]>) -> Self {
        Self {
            teams: teams.as_ref().to_vec(),
            fields: fields.as_ref().to_vec(),
        }
    }
}

pub fn schedule<K>(_input: ScheduledInput) -> Result<()> {
    algorithm::v2::test()
}
