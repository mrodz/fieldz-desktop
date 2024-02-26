pub mod algorithm;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Local, TimeDelta, TimeZone, Timelike};
use petgraph::{
    data::Build,
    graphmap::{DiGraphMap, GraphMap},
    Directed,
};
use rand::{rngs::ThreadRng, Rng};

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

#[derive(Debug, Default)]
pub struct League {
    regions: Vec<Pin<Box<Region>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledOutput {
    valid_games: HashSet<Reservation>,
    unable_to_schedule: Vec<Reservation>,
    unplayed_matches: Vec<Game>,
}

impl ScheduledOutput {
    #[allow(clippy::mutable_key_type)]
    pub fn valid_games(&self) -> &HashSet<Reservation> {
        &self.valid_games
    }
    pub fn unable_to_schedule(&self) -> &[Reservation] {
        &self.unable_to_schedule
    }
    pub fn unplayed_matches(&self) -> &[Game] {
        &self.unplayed_matches
    }
}

struct RandomEdgeSupplier<'a> {
    vec: Vec<(Pin<&'a Team>, Pin<&'a Team>, &'a Option<&'a Reservation>)>,
    rng: ThreadRng,
}

impl<'a> Iterator for RandomEdgeSupplier<'a> {
    fn next(&mut self) -> Option<Self::Item> {
        if self.vec.is_empty() {
            return None;
        }

        let index = self.rng.gen_range(0..self.vec.len());
        Some(self.vec.swap_remove(index))
    }
    type Item = (Pin<&'a Team>, Pin<&'a Team>, &'a Option<&'a Reservation>);
}

impl League {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_region(&mut self, region: Region) {
        self.regions.push(Box::pin(region));
    }

    pub fn schedule(&mut self) -> Result<ScheduledOutput> {
        // Safe because a Reservation's hash is not determined by any interior mutability
        #[allow(clippy::mutable_key_type)]
        let mut set = HashSet::new();

        let mut unable_to_schedule = Vec::new();

        let mut unplayed_matches: Vec<Game> = Vec::new();

        for region in self.regions.iter_mut() {
            let mut graph = region.season_graph();
            let mut time_queue = RegionalGameQueue::new(region);

            let mut next_window = time_queue.next();

            let mut busy: HashMap<Pin<&Team>, Vec<&AvailabilityWindow>> = HashMap::new();
            let iter_access_point: *mut GraphMap<_, _, _> = &mut graph as *mut _;

            let mut edges = graph.all_edges();

            while let (Some((mut home, mut away, edge)), Some(next_window_view)) =
                (edges.next(), next_window)
            {
                println!("\n-- BEGIN TRANSACTION\n{home}, {away}, {edge:?}");
                if edge.is_some() {
                    println!("\talready set\n--END TRANSACTION");
                    // we can ignore this edge, since it was already set
                    continue;
                }

                let collision_fn = |window: &&AvailabilityWindow| {
                    AvailabilityWindow::overlap(
                        (*window).clone(),
                        next_window_view.availability_window.clone(),
                        TimeDelta::new(0, 0).unwrap(),
                    )
                    .unwrap()
                };

                if let Some(home_times) = busy.get(&home) {
                    if home_times.iter().any(collision_fn) {
                        println!(
                            "\t{} <home> is busy at {}, finding more opponents:",
                            home, next_window_view.availability_window
                        );
                        let other_matches = graph.edges(away);
                        let mut unschedulable = true;
                        for (_, new_home, reservation) in other_matches {
                            if let Some(reservation) = reservation {
                                println!(
                                    "\t\t{} v {} was already scheduled ({reservation})",
                                    new_home, away
                                );
                                continue;
                            }

                            if busy
                                .get(&new_home)
                                .is_some_and(|times| times.iter().any(collision_fn))
                            {
                                println!(
                                    "\t\t{} cannot play as home because they're booked at {}",
                                    new_home, next_window_view.availability_window
                                );
                                continue;
                            }

                            home = new_home;
                            unschedulable = false;
                            println!("\t\t\tFound new home team! {home}");
                            break;
                        }

                        if unschedulable {
                            println!("\tWasting space!");
                            continue;
                        }
                    }
                }

                if let Some(away_times) = busy.get(&away) {
                    if away_times.iter().any(collision_fn) {
                        println!(
                            "\t{} <away> is busy at {}, finding more opponents:",
                            away, next_window_view.availability_window
                        );
                        let other_matches = graph.edges(home);
                        let mut unschedulable = true;
                        for (_, new_away, reservation) in other_matches {
                            if let Some(reservation) = reservation {
                                println!(
                                    "\t\t{} v {} was already scheduled ({reservation})",
                                    home, new_away
                                );
                                continue;
                            }

                            if busy
                                .get(&new_away)
                                .is_some_and(|times| times.iter().any(collision_fn))
                            {
                                println!(
                                    "\t\t{} cannot play as away because they're booked at {}",
                                    new_away, next_window_view.availability_window
                                );
                                continue;
                            }

                            away = new_away;
                            unschedulable = false;
                            println!("\t\tFound new away team! {away}");
                            break;
                        }

                        if unschedulable {
                            println!("\tWasting space!\n--END TRANSACTION");
                            unable_to_schedule.push(next_window_view.clone());
                            next_window = time_queue.next();
                            continue;
                        }
                    }
                }

                let game = Game::new(home.deref(), away.deref());
                next_window_view.set_game(game);

                println!("\tLet's go! {next_window_view}");

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

                busy.entry(home)
                    .and_modify(|v| v.push(&next_window_view.availability_window))
                    .or_insert(vec![&next_window_view.availability_window]);
                busy.entry(away)
                    .and_modify(|v| v.push(&next_window_view.availability_window))
                    .or_insert(vec![&next_window_view.availability_window]);

                next_window = time_queue.next();

                println!("-- END TRANSACTION")
            }

            set.extend(graph.all_edges().filter_map(|(.., r)| r.cloned()));
            unable_to_schedule.append(&mut time_queue.map(Reservation::clone).collect());

            // if we exit early, we have to check if the last window was fulfilled.
            if next_window.is_some_and(Reservation::is_available) {
                unable_to_schedule.push(next_window.unwrap().clone())
            }

            unplayed_matches.append(
                &mut graph
                    .all_edges()
                    .filter_map(|(home, away, reservation)| {
                        if reservation.is_none() {
                            Some(Game::new(home.deref(), away.deref()))
                        } else {
                            None
                        }
                    })
                    .collect(),
            );
        }

        let result = ScheduledOutput {
            valid_games: set,
            unable_to_schedule,
            unplayed_matches,
        };

        Ok(result)
    }

    pub fn teams(&self) -> Vec<&Team> {
        let mut result = vec![];
        for region in &self.regions {
            result.append(&mut region.teams());
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Region {
    fields: Vec<Pin<Box<Field>>>,
    teams: Vec<Pin<Box<Team>>>,
}

#[derive(Clone)]
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
            ..Default::default()
        }
    }

    pub fn add_team(&mut self, identifier: impl Into<String>) {
        let region = NonNull::from(self.deref());
        self.teams.push(Box::pin(Team {
            identifier: identifier.into(),
            region,
        }))
    }

    pub fn add_field(
        &mut self,
        identifier: impl Into<String>,
        availability: &[AvailabilityWindow],
    ) {
        let region = NonNull::new(self as *mut _).unwrap();

        let matches = VecDeque::from_iter(
            availability
                .iter()
                .map(|w| Reservation::new(w.clone(), NonNull::dangling())),
        );

        let mut pinned_field = Box::pin(Field {
            identifier: identifier.into(),
            region,
            matches,
        });

        let cyclical_self = NonNull::from(pinned_field.deref());

        pinned_field
            .matches
            .iter_mut()
            .for_each(|r| r.field = cyclical_self);

        self.fields.push(pinned_field)
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

    pub fn teams(&self) -> Vec<&Team> {
        let mut result = vec![];
        for team in &self.teams {
            result.push(team.deref());
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Team {
    identifier: String,
    region: NonNull<Region>,
}

impl Hash for Team {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.identifier.hash(state);
        self.region.hash(state);
    }
}

impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

#[macro_export]
macro_rules! window {
    ($dd:literal/$mm:literal/$yy:literal from $start_h:literal:$start_m:literal to $end_h:literal:$end_m:literal) => {
        $crate::AvailabilityWindow::single_day(
            $dd,
            $mm,
            $yy,
            ($start_h, $start_m),
            ($end_h, $end_m),
        )
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
            .with_ymd_and_hms(
                year.try_into()?,
                month,
                day,
                start.0,
                start.1,
                0,
            )
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

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

#[derive(Clone, Debug)]
pub struct Reservation {
    availability_window: AvailabilityWindow,
    game: Cell<Option<Game>>,
    field: NonNull<Field>,
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.availability_window == other.availability_window && self.field == other.field
    }
}

impl Eq for Reservation {}

impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(f, "{} [{}]", self.availability_window, self.field.as_ref())?;
        }

        if let Some(game) = self.game.get() {
            write!(f, " {game}")
        } else {
            write!(f, " <unused>")
        }
    }
}

impl Hash for Reservation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.availability_window.hash(state);
        self.field.hash(state);
    }
}

impl Reservation {
    pub const fn new(availability_window: AvailabilityWindow, field: NonNull<Field>) -> Self {
        Self {
            availability_window,
            game: Cell::new(None),
            field,
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

    pub fn time(&self) -> &AvailabilityWindow {
        &self.availability_window
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
