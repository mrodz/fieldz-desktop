pub mod algorithm;

use std::fmt::Display;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Local, TimeDelta, TimeZone, Timelike};

#[derive(Debug, Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct AvailabilityWindow {
    start: DateTime<Local>,
    end: DateTime<Local>,
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
        let start = Local
            .with_ymd_and_hms(year.try_into()?, month, day, start.0, start.1, 0)
            .earliest()
            .context("ambiguous start date")?;
        let end = Local
            .with_ymd_and_hms(year.try_into()?, month, day, end.0, end.1, 0)
            .earliest()
            .context("ambiguous end date")?;

        if end < start {
            bail!("End time ({end:?}) is before ({start:?})");
        }

        Ok(Self { start, end })
    }

    pub fn new(start: DateTime<Local>, end: DateTime<Local>) -> Result<Self> {
        if end < start {
            bail!("End time ({end:?}) is before ({start:?})");
        }

        Ok(Self { start, end })
    }

    pub fn contains(&self, time: DateTime<Local>) -> bool {
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

    pub fn as_unix(&self) -> Result<(i32, i32)> {
        Ok((
            self.start.timestamp().try_into()?,
            self.end.timestamp().try_into()?,
        ))
    }
}

trait TeamLike {
    fn id(&self) -> u8;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Team {
    name: Option<String>
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Field {
    name: Option<String>,
    time_slots: Vec<AvailabilityWindow>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ScheduledInput
{
    teams: Vec<Team>,
    fields: Vec<Field>,
}

impl ScheduledInput
{
    pub fn new(teams: impl AsRef<[Team]>, fields: impl AsRef<[Field]>) -> Self
    {
        Self {
            teams: teams.as_ref().to_vec(),
            fields: fields.as_ref().to_vec(),
        }
    }
}

pub fn schedule<K>(input: ScheduledInput) -> Result<()>
{
    algorithm::v2::test()
    // if input.teams.len() <= 3 {
    //     let mut league = algorithm::v1::League::new();

    //     let mut region = algorithm::v1::Region::new();

    //     for (field, availability) in input.availability {
    //         region.add_field(field, &availability);
    //     }

    //     for team in input.teams {
    //         region.add_team(team.id().to_string());
    //     }

    //     league.add_region(region);

    //     let out = league.schedule()?;

        
    // }
}
