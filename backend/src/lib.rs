pub mod algorithm;

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
};

use anyhow::{bail, Context, Result};
use chrono::{serde::ts_seconds, DateTime, Datelike, TimeDelta, TimeZone, Timelike, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Hash, PartialEq, Eq, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AvailabilityWindow {
    #[serde(with = "ts_seconds")]
    start: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    end: DateTime<Utc>,
}

pub trait TeamLike {
    fn unique_id(&self) -> i32;
}

pub type ProtobufAvailabilityWindow = (i64, i64);

pub trait FieldLike {
    fn unique_id(&self) -> i32;
    fn time_slots(&self) -> impl AsRef<[(ProtobufAvailabilityWindow, u8)]>;
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

    pub fn new_unix(start: i64, end: i64) -> Result<Self> {
        Self::new(
            DateTime::from_timestamp(start, 0).context("start")?,
            DateTime::from_timestamp(end, 0).context("end")?,
        )
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

    pub(crate) fn as_lossy_window(
        &self,
        compression_profile: &CompressionProfile,
    ) -> Result<LossyAvailability, LossyAvailabilityError> {
        LossyAvailability::new(&self.start, &self.end, compression_profile)
    }

    pub const fn to_protobuf_window(&self) -> ProtobufAvailabilityWindow {
        (self.start.timestamp(), self.end.timestamp())
    }
}

#[cfg(test)]
mod compressed_availability_window {
    use chrono::{TimeZone, Utc};

    use crate::{window, CompressionProfile};

    #[test]
    fn normal_normal() {
        let time = window!(10/2/2023 from 8:00 to 9:00).unwrap();

        let profile = CompressionProfile::assume_date(
            &Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0)
                .earliest()
                .unwrap(),
        );

        let compressed = time.as_lossy_window(&profile).unwrap();

        let reverted = compressed.as_availability_window_lossy(&profile);

        assert_eq!(time, reverted);
    }

    #[test]
    fn normal_half() {
        let time = window!(10/2/2023 from 8:00 to 9:30).unwrap();

        let profile = CompressionProfile::assume_date(
            &Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0)
                .earliest()
                .unwrap(),
        );

        let compressed = time.as_lossy_window(&profile).unwrap();

        let reverted = compressed.as_availability_window_lossy(&profile);

        assert_eq!(time, reverted);
    }

    #[test]
    fn half_normal() {
        let time = window!(10/2/2023 from 8:30 to 9:00).unwrap();

        let profile = CompressionProfile::assume_date(
            &Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0)
                .earliest()
                .unwrap(),
        );

        let compressed = time.as_lossy_window(&profile).unwrap();

        let reverted = compressed.as_availability_window_lossy(&profile);

        assert_eq!(time, reverted);
    }

    #[test]
    fn half_half() {
        let time = window!(10/2/2023 from 8:30 to 9:30).unwrap();

        let profile = CompressionProfile::assume_date(
            &Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0)
                .earliest()
                .unwrap(),
        );

        let compressed = time.as_lossy_window(&profile).unwrap();

        let reverted = compressed.as_availability_window_lossy(&profile);

        assert_eq!(time, reverted);
    }

    #[test]
    fn lost_data() {
        // 8:15 will round up to 8:30
        let time = window!(10/2/2023 from 8:15 to 9:30).unwrap();

        let profile = CompressionProfile::assume_date(
            &Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0)
                .earliest()
                .unwrap(),
        );

        let compressed = time.as_lossy_window(&profile).unwrap();

        let reverted = compressed.as_availability_window_lossy(&profile);

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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct CompressionProfile {
    /// UNIX timestamp
    earliest_date: i64,
}

impl CompressionProfile {
    pub fn assume_date(time_slot: &DateTime<Utc>) -> Self {
        Self {
            earliest_date: time_slot.timestamp(),
        }
    }
}

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
    pub fn new(
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        compression_profile: &CompressionProfile,
    ) -> Result<Self, LossyAvailabilityError> {
        let start_milliseconds: i64 = start.timestamp() - compression_profile.earliest_date;

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

        let end_milliseconds: i64 = end.timestamp() - compression_profile.earliest_date;

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

    fn as_datetime_lossy(
        &self,
        compression_profile: &CompressionProfile,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
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

        let start_hours =
            DateTime::from_timestamp(start_seconds + compression_profile.earliest_date, 0).unwrap();
        let end_hours =
            DateTime::from_timestamp(end_seconds + compression_profile.earliest_date, 0).unwrap();

        (start_hours, end_hours)
    }

    /// This function will decompress the time window.
    pub fn as_availability_window_lossy(
        &self,
        compression_profile: &CompressionProfile,
    ) -> AvailabilityWindow {
        let (start, end) = self.as_datetime_lossy(compression_profile);
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Team {
    name: String,
    id: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Field {
    id: u8,
    name: Option<Box<str>>,
    time_slots: Vec<(AvailabilityWindow, ReservationType)>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReservationType(u8);

pub trait PlayableTeamCollection
where
    Self: Clone + Debug + PartialEq,
{
    type Team: TeamLike;

    fn teams(&self) -> impl AsRef<[Self::Team]>;
}

pub trait CoachConflictLike
where
    Self: Clone + Debug + PartialEq,
{
    type Team: TeamLike;

    fn teams(&self) -> impl AsRef<[Self::Team]>;
    fn unique_id(&self) -> i32;
    fn region_id(&self) -> i32;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledInput<T, P, F, C>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    team_groups: Vec<P>,
    fields: Vec<F>,
    unique_id: i32,
    coach_conflicts: Vec<C>,
}

impl<T, P, F, C> ScheduledInput<T, P, F, C>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    pub fn new(
        unique_id: i32,
        teams: impl AsRef<[P]>,
        fields: impl AsRef<[F]>,
        coach_conflicts: impl AsRef<[C]>,
    ) -> Self {
        Self {
            unique_id,
            team_groups: teams.as_ref().to_vec(),
            fields: fields.as_ref().to_vec(),
            coach_conflicts: coach_conflicts.as_ref().to_vec(),
        }
    }

    pub fn get_compression_profile(&self) -> Result<Option<CompressionProfile>> {
        let earliest_date = self
            .fields
            .iter()
            .flat_map(|field| {
                field
                    .time_slots()
                    .as_ref()
                    .iter()
                    .map(|(time_slot, _)| time_slot.0)
                    .collect_vec()
            })
            .minmax();

        match earliest_date {
            itertools::MinMaxResult::NoElements => Ok(None),
            itertools::MinMaxResult::OneElement(element) => Ok(Some(CompressionProfile {
                earliest_date: element,
            })),
            itertools::MinMaxResult::MinMax(min, max) => {
                const SECONDS_IN_15_BIT_HOUR_MAX: i64 = ((1 << 15) - 1) * SECONDS_TO_HOURS;

                if let 2..=SECONDS_IN_15_BIT_HOUR_MAX = max - min {
                    Ok(Some(CompressionProfile { earliest_date: min }))
                } else {
                    bail!("Cannot use date compression, as the breadth of input time slots exceeds {SECONDS_IN_15_BIT_HOUR_MAX} seconds (~3.7 years): {min} & {max}");
                }
            }
        }
    }

    fn into_transformer(
        self,
        compression_profile: &CompressionProfile,
    ) -> Result<StateTransformer<T, P, F>> {
        let mut result = algorithm::v2::MCTSState::new();

        // We need to use a hashmap because team with a high id would overflow.
        let mut scheduler_field_id_to_field_id = HashMap::new();

        for (this_field_id, field) in self.fields.iter().enumerate() {
            let time_slots = field.time_slots();
            let time_slots = time_slots.as_ref();
            let mut availability = Vec::with_capacity(time_slots.len());

            for (time_slot, supported_concurrency) in time_slots {
                for _ in 0..*supported_concurrency {
                    availability.push(*time_slot);
                }
            }

            let byte = this_field_id.try_into().expect("too many fields (>= 256)");

            let mut as_windows = Vec::with_capacity(availability.len());

            for (start, end) in availability {
                as_windows.push(AvailabilityWindow::new_unix(start, end)?);
            }

            result.add_time_slots(byte, as_windows, compression_profile);
            scheduler_field_id_to_field_id.insert(byte, field.unique_id());
        }

        // We need to use a hashmap because the sequentiality of a team's ID is not guaranteed,
        // but depended on by the algorithm.
        let mut playable_group_index_to_team_index = HashMap::new();
        // We create two hashmaps because there is no standard bidirectional map in Rust.
        let mut team_index_to_playable_group_index = HashMap::new();

        let mut this_team_index = 0;

        for team_group in &self.team_groups {
            let mut playable_group = algorithm::v2::PlayableGroup::new(this_team_index);

            for team in team_group.teams().as_ref() {
                let g_id = this_team_index.try_into()?;
                playable_group_index_to_team_index.insert(g_id, team.unique_id());
                team_index_to_playable_group_index.insert(team.unique_id(), g_id);
                playable_group.add_team(g_id);
                this_team_index += 1;
            }

            result.add_group(playable_group);
        }

        for coach_conflict in self.coach_conflicts() {
            result.add_team_collisions(
                coach_conflict.teams(),
                Some(&team_index_to_playable_group_index),
            )
        }

        Ok(StateTransformer {
            inner: result,
            playable_group_index_to_team_index,
            team_groups: self.team_groups,
            scheduler_field_id_to_field_id,
            fields: self.fields,
            unique_id: self.unique_id,
        })
    }

    pub fn fields(&self) -> &[F] {
        &self.fields
    }

    pub fn team_groups(&self) -> &[P] {
        &self.team_groups
    }

    pub fn coach_conflicts(&self) -> &[C] {
        &self.coach_conflicts
    }

    pub fn teams_len(&self) -> usize {
        let mut result = 0;
        for team_group in &self.team_groups {
            result += team_group.teams().as_ref().len();
        }
        result
    }
}

#[derive(Debug)]
struct StateTransformer<T, P, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    P: PlayableTeamCollection<Team = T>,
    F: FieldLike + Clone + Debug + PartialEq,
{
    unique_id: i32,
    inner: algorithm::v2::MCTSState,
    playable_group_index_to_team_index: HashMap<u8, i32>,
    team_groups: Vec<P>,
    scheduler_field_id_to_field_id: HashMap<u8, i32>,
    fields: Vec<F>,
}

impl<T, P, F> StateTransformer<T, P, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    P: PlayableTeamCollection<Team = T>,
    F: FieldLike + Clone + Debug + PartialEq,
{
    fn scheduler_state(&self) -> &algorithm::v2::MCTSState {
        &self.inner
    }

    fn team_from_schedule_id(&self, schedule_id: u8) -> Option<T> {
        let searching_for = *self.playable_group_index_to_team_index.get(&schedule_id)?;

        for team_group in &self.team_groups {
            if let Some(result) = team_group
                .teams()
                .as_ref()
                .iter()
                .find(|team| team.unique_id() == searching_for)
            {
                return Some(result.clone());
            }
        }

        None
    }

    fn field_from_schedule_id(&self, schedule_id: u8) -> Option<&F> {
        let searching_for = *self.scheduler_field_id_to_field_id.get(&schedule_id)?;

        self.fields
            .iter()
            .find(|field| field.unique_id() == searching_for)
    }

    fn transform_v2_reservation(
        &self,
        reservation: &algorithm::v2::Reservation,
        compression_profile: &CompressionProfile,
    ) -> Reservation<T, F> {
        Reservation {
            field: self
                .field_from_schedule_id(reservation.slot().field_id())
                .expect("field was not mapped properly")
                .clone(),
            availability: reservation
                .slot()
                .availability()
                .as_availability_window_lossy(compression_profile),
            booking: match reservation.game() {
                Some(game) => Booking::Booked {
                    home_team: self
                        .team_from_schedule_id(game.team_one().id())
                        .expect("team was not mapped properly"),
                    away_team: self
                        .team_from_schedule_id(game.team_two().id())
                        .expect("team was not mapped properly"),
                },
                None => Booking::Empty,
            },
        }
    }

    fn transform_v2(
        self,
        input: algorithm::v2::Output,
        compression_profile: &CompressionProfile,
    ) -> Output<T, F> {
        let mut time_slots = vec![];

        for reservation in input.reservations() {
            time_slots.push(self.transform_v2_reservation(reservation, compression_profile));
        }

        Output {
            time_slots,
            unique_id: self.unique_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Booking<T>
where
    T: TeamLike + Clone + Debug + PartialEq,
{
    Booked { home_team: T, away_team: T },
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reservation<T, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    F: FieldLike + Clone + Debug + PartialEq,
{
    field: F,
    availability: AvailabilityWindow,
    booking: Booking<T>,
}

impl<T, F> Reservation<T, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    F: FieldLike + Clone + Debug + PartialEq,
{
    pub const fn field(&self) -> &F {
        &self.field
    }

    pub const fn start(&self) -> i64 {
        self.availability.start.timestamp()
    }

    pub const fn end(&self) -> i64 {
        self.availability.end.timestamp()
    }

    pub const fn booking(&self) -> &Booking<T> {
        &self.booking
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Output<T, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    F: FieldLike + Clone + Debug + PartialEq,
{
    time_slots: Vec<Reservation<T, F>>,
    unique_id: i32,
}

impl<T, F> Output<T, F>
where
    T: TeamLike + Clone + Debug + PartialEq,
    F: FieldLike + Clone + Debug + PartialEq,
{
    pub fn time_slots(&self) -> &[Reservation<T, F>] {
        &self.time_slots
    }

    pub const fn unique_id(&self) -> i32 {
        self.unique_id
    }
}

pub fn schedule<T, P, F, C>(input: ScheduledInput<T, P, F, C>) -> Result<Output<T, F>>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    let Some(compression_profile) = input.get_compression_profile()? else {
        return Ok(Output {
            time_slots: input
                .fields()
                .iter()
                .flat_map(|field| {
                    field
                        .time_slots()
                        .as_ref()
                        .iter()
                        .map(|(time_slot, _)| Reservation {
                            field: field.clone(),
                            availability: AvailabilityWindow::new_unix(time_slot.0, time_slot.1)
                                .expect("field availability"),
                            booking: Booking::Empty,
                        })
                        .collect_vec()
                })
                .collect_vec(),
            unique_id: input.unique_id,
        });
    };
    let transformer = input.into_transformer(&compression_profile)?;
    let output = algorithm::v2::schedule(transformer.scheduler_state())?;
    Ok(transformer.transform_v2(output, &compression_profile))
}
