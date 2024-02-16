use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use petgraph::{
    graphmap::{DiGraphMap, GraphMap},
    Directed,
};

use std::{
    cmp::Ordering,
    collections::{HashSet, VecDeque},
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
        let set = HashSet::new();

        for region in self.regions.iter_mut() {
            let graph = region.season_graph();
            println!("{graph:#?}");
        }

        set
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
        let region = NonNull::new(self as *mut _).unwrap();
        let matches = VecDeque::from_iter(availability.iter().map(Reservation::new));

        self.fields.push(Field {
            identifier: identifier.into(),
            region,
            matches,
        })
    }

    pub fn season_graph(
        &mut self,
    ) -> GraphMap<Pin<&Team>, Option<AvailabilityWindow>, Directed> {
        let mut graph = DiGraphMap::new();

        for (i, home) in self.teams.iter().enumerate() {
            println!("home = {home:?}");
            graph.add_node(home.as_ref());

            for (j, away) in self.teams.iter().enumerate() {
                if j == i {
                    continue;
                }

                println!("\taway = {away:?} on {home:?}");

                graph.add_edge(home.as_ref(), away.as_ref(), None);
            }
        }

        graph
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Team {
    identifier: String,
    region: NonNull<Region>,
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
    matches: VecDeque<Reservation>,
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.identifier.partial_cmp(&other.identifier)
    }
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier && self.region.as_ptr() == other.region.as_ptr()
    }
}

impl Eq for Field {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Reservation {
    availability_window: AvailabilityWindow,
    game: Option<Game>,
}

impl Reservation {
    pub fn new(availability_window: &AvailabilityWindow) -> Self {
        Self {
            availability_window: availability_window.to_owned(),
            game: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Game {
    home: NonNull<Team>,
    away: NonNull<Team>,
}

impl Game {}
