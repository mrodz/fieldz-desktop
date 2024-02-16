use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, TimeZone};

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashSet},
    hash::Hash,
    pin::Pin,
    ptr::NonNull,
};

#[derive(Debug)]
pub struct League {
    regions: Vec<Pin<Box<Region>>>,
}

impl League {
    pub fn new() -> Self {
        Self { regions: vec![] }
    }

    pub fn add_region(&mut self, region: Region) {
        self.regions.push(Box::pin(region));
    }

    pub fn schedule(&mut self) -> HashSet<Reservation> {
        let mut set = HashSet::new();
        for region in self.regions.iter_mut() {
            set.extend(region.schedule_own_games());
        }

        set
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Region {
    fields: Vec<Field>,
    teams: Vec<Pin<Box<Team>>>,
}

impl Region {
    pub fn new() -> Self {
        Self {
            fields: vec![],
            teams: vec![],
        }
    }

    pub fn add_team(&mut self, identifier: impl Into<String>) {
        let self_ref = self as *mut _;

        self.teams.push(Box::pin(Team {
            identifier: identifier.into(),
            region: NonNull::new(self_ref).unwrap(),
        }))
    }

    pub fn add_field(
        &mut self,
        identifier: impl Into<String>,
        availability: &[AvailabilityWindow],
    ) {
        let self_ref = self as *mut _;

        let mut matches = BinaryHeap::with_capacity(availability.len());

        for window in availability {
            matches.push(Reservation {
                availability_window: window.clone(),
                game: None,
            });
        }

        self.fields.push(Field {
            identifier: identifier.into(),
            region: NonNull::new(self_ref).unwrap(),
            matches,
        })
    }

    pub fn schedule_own_games(&mut self) -> HashSet<Reservation> {
		let result = HashSet::new();

        for team in &self.teams {
			todo!();
		}

		result
    }
}

#[derive(Debug)]
pub struct Team {
    identifier: String,
    region: NonNull<Region>,
}

impl PartialEq for Team {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
            && unsafe { self.region.as_ref() == other.region.as_ref() }
    }
}

impl Eq for Team {}

impl Hash for Team {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.identifier.hash(state);
    }
}

#[macro_export]
macro_rules! window {
    ($dd:literal/$mm:literal/$yy:literal from $start_h:literal:$start_m:literal to $end_h:literal:$end_m:literal) => {
        AvailabilityWindow::single_day($dd, $mm, $yy, ($start_h, $start_m), ($end_h, $end_m))
    };
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct AvailabilityWindow {
    start: DateTime<Local>,
    end: DateTime<Local>,
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
            .single()
            .context("ambiguous start date")?;
        let end = Local
            .with_ymd_and_hms(year.try_into()?, month, day, end.0, end.1, 0)
            .single()
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
}

#[derive(Debug)]
pub struct Field {
    identifier: String,
    region: NonNull<Region>,
    matches: BinaryHeap<Reservation>,
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.identifier == other.identifier && self.region.as_ref() == other.region.as_ref()
        }
    }
}

impl Eq for Field {}

#[derive(Debug, Hash)]
pub struct Reservation {
    availability_window: AvailabilityWindow,
    game: Option<Game>,
}

impl Eq for Reservation {}

impl Ord for Reservation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Reservation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.game, &other.game) {
            (None, Some(..)) => Some(Ordering::Greater),
            (Some(..), None) => Some(Ordering::Less),
            (None, None) => Some(Ordering::Equal),
            _ => self
                .availability_window
                .partial_cmp(&other.availability_window),
        }
    }
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).is_some_and(Ordering::is_eq)
    }
}

#[derive(Debug)]
pub struct Game {
    home: NonNull<Team>,
    away: NonNull<Team>,
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.home.as_ref() == other.home.as_ref() && self.away.as_ref() == other.away.as_ref()
        }
    }
}

impl Hash for Game {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe {
            self.home.as_ref().hash(state);
        }
    }
}

impl Game {}
