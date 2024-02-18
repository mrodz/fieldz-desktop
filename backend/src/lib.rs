use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Local, TimeDelta, TimeZone, Timelike};
use petgraph::{
    data::Build,
    graphmap::{DiGraphMap, GraphMap},
    Directed,
};

use std::{
    cell::Cell,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    hash::Hash,
    ops::Deref,
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

    pub fn schedule<'a>(&'a mut self) -> HashSet<Reservation> {
        let mut set = HashSet::new();

        for region in self.regions.iter_mut() {
            let mut graph = region.season_graph();
            let mut time_queue = RegionalGameQueue::new(&region);

            let mut next_window = time_queue.next();

            let mut busy: HashMap<Pin<&Team>, AvailabilityWindow> = HashMap::new();
            let iter_access_point: *mut GraphMap<_, _, _> = &mut graph as *mut _;
            let mut edges = graph.all_edges();

            while let (Some((home, away, edge)), Some(next_window_view)) =
                (edges.next(), next_window)
            {
                if edge.is_some() {
                    // we can ignore this edge, since it was already set
                    continue;
                }

                busy.retain(|_, window| {
                    AvailabilityWindow::overlap(
                        next_window_view.availability_window.clone(),
                        window.clone(),
                        TimeDelta::new(0, 0).unwrap(),
                    )
                    .expect(">:(")
                });

                if busy.contains_key(&home) || busy.contains_key(&away) {
                    continue;
                } else {
                    let game = Game::new(home.deref(), away.deref());
                    next_window_view.set_game(game);

                    /*
                     * This is actually safe behavior, because we want our edits to be present
                     * in future iterations.
                     *
                     * Rust does not know that it is safe to mutate the edge's weight (because the
                     * iterator is borrowed immutably), so we use a pointer to bypass the borrow checker.
                     */
                    unsafe {
                        (*iter_access_point).update_edge(home, away, Some(next_window_view));
                    }
                    next_window = time_queue.next();
                }
            }

            set.extend(graph.all_edges().filter_map(|x| x.2.cloned()))
        }

        set
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Region {
    fields: Vec<Field>,
    teams: Vec<Pin<Box<Team>>>,
}

pub struct RegionalGameQueue<'a> {
    region: &'a Region,
    index: (usize, usize),
}

impl<'a> RegionalGameQueue<'a> {
    pub const fn new(region: &'a Region) -> Self {
        Self {
            region,
            index: (0, 0),
        }
    }
}

impl<'a> Iterator for RegionalGameQueue<'a> {
    type Item = &'a Reservation;

    fn next(&mut self) -> Option<Self::Item> {
        let field = self.region.fields.get(self.index.0)?;

        fn rollover(i: &mut (usize, usize)) {
            i.0 += 1;
            i.1 = 0;
        }

        let Some(reservation) = field.matches.get(self.index.1) else {
            rollover(&mut self.index);
            return self.next();
        };

        self.index.1 += 1;
        if self.index.1 > field.matches.len() {
            rollover(&mut self.index);
        }

        if reservation.is_available() {
            Some(reservation)
        } else {
            self.next()
        }
    }
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

    pub fn season_graph(&self) -> GraphMap<Pin<&Team>, Option<&Reservation>, Directed> {
        let mut graph = DiGraphMap::new();

        for (i, home) in self.teams.iter().enumerate() {
            graph.add_node(home.as_ref());

            for (j, away) in self.teams.iter().enumerate() {
                if j != i {
                    graph.add_edge(home.as_ref(), away.as_ref(), None);
                }
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

impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
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

    pub fn overlap(mut lhs: Self, mut rhs: Self, err: TimeDelta) -> Result<bool> {
        lhs.add_error(err)?;
        rhs.add_error(err)?;

        Ok(lhs.start <= rhs.end && lhs.end >= rhs.start)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservation {
    availability_window: AvailabilityWindow,
    game: Cell<Option<Game>>,
}

impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(game) = self.game.get() {
            write!(f, "{game}")?;
        } else {
            write!(f, "<empty time slot>")?;
        }

        write!(f, " at {}", self.availability_window)
    }
}

impl Hash for Reservation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.availability_window.hash(state);
        self.game.get().hash(state);
    }
}

impl Reservation {
    pub fn new(availability_window: &AvailabilityWindow) -> Self {
        Self {
            availability_window: availability_window.to_owned(),
            game: Cell::new(None),
        }
    }

    pub fn set_game(&self, game: Game) -> Option<Game> {
        let copy = self.game.get();

        self.game.set(Some(game));

        if copy.is_some() {
            copy
        } else {
            None
        }
    }

    pub fn is_available(&self) -> bool {
        self.game.get().is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Game {
    home: NonNull<Team>,
    away: NonNull<Team>,
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "{} vs {}",
                self.home.as_ref().identifier,
                self.away.as_ref().identifier
            )
        }
    }
}

impl Game {
    pub fn new(home: &Team, away: &Team) -> Self {
        Self {
            home: NonNull::from(home),
            away: NonNull::from(away),
        }
    }
}
