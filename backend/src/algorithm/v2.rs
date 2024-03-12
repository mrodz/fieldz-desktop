extern crate tinyvec;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::num::NonZeroU8;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use itertools::Itertools;
use itertools::MinMaxResult;
use mcts::transposition_table::*;
use mcts::tree_policy::*;
use mcts::*;
use tinyvec::tiny_vec;
use tinyvec::TinyVec;

use crate::window;
use crate::AvailabilityWindow;
use crate::LossyAvailability;

type TeamId = u8;

#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Team {
    id: TeamId,
}

impl Team {
    fn new(id: TeamId) -> Self {
        Self { id }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlayableGroup {
    teams: TinyVec<[TeamSlot; 8]>,
    external_id: Option<NonZeroU8>,
    index_start: usize,
}

impl PlayableGroup {
    pub fn new(index_start: usize) -> Self {
        Self {
            teams: tiny_vec![],
            external_id: None,
            index_start,
        }
    }

    pub fn get_team(&mut self, id: TeamId) -> &mut TeamSlot {
        &mut self.teams[id as usize - self.index_start]
    }

    pub fn add_team(&mut self, id: TeamId) {
        self.teams.push(TeamSlot(Team::new(id), tiny_vec![]));
    }

    pub fn set_index(&mut self, external_id: NonZeroU8) {
        assert!(
            self.external_id.replace(external_id).is_none(),
            "ID was already set"
        );
    }

    #[inline(always)]
    pub fn id(&self) -> NonZeroU8 {
        self.external_id.unwrap()
    }
}

impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Team {}", self.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Game {
    team_one: Team,
    team_two: Team,
    group_id: NonZeroU8,
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} v {}", self.team_one, self.team_two)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Reservation {
    slot: Slot,
    game: Option<Game>,
}

impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slot)?;

        if let Some(game) = self.game.as_ref() {
            write!(f, " @ {game}")
        } else {
            write!(f, " <wasted>")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Slot {
    field_id: u8,
    availability: LossyAvailability,
}

impl Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[f{}] {}",
            self.field_id,
            self.availability.as_availability_window_lossy(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TeamSlot(Team, TinyVec<[Slot; 8]>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MCTSState {
    games: BTreeMap<Slot, Option<Game>>,
    groups: Vec<PlayableGroup>,
    teams_len: u8,
}

impl MCTSState {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_group(&mut self, mut playable_group: PlayableGroup) {
        playable_group.set_index(
            NonZeroU8::new(<usize as TryInto<u8>>::try_into(self.groups.len()).unwrap() + 1)
                .unwrap(),
        );
        self.teams_len = self
            .teams_len
            .checked_add(
                playable_group
                    .teams
                    .len()
                    .try_into()
                    .expect("max team size = 255"),
            )
            .expect("max team size = 255");
        self.groups.push(playable_group);
    }

    pub fn add_time_slots(&mut self, field_id: u8, time_slots: impl AsRef<[AvailabilityWindow]>) {
        for time_slot in time_slots.as_ref() {
            assert!(
                self.games
                    .insert(
                        Slot {
                            field_id,
                            availability: time_slot.as_lossy_window().unwrap()
                        },
                        None
                    )
                    .is_none(),
                "{time_slot} at field {field_id} was already set"
            );
        }
    }

    pub const fn teams_len(&self) -> u8 {
        self.teams_len
    }
}

impl GameState for MCTSState {
    type Move = Reservation;
    type Player = ();
    type MoveList = Vec<Reservation>;

    fn current_player(&self) -> Self::Player {}

    #[inline(always)]
    fn available_moves(&self) -> Vec<Reservation> {
        let mut result = tiny_vec!([Reservation; 8]);

        for (slot, game) in &self.games {
            if game.is_some() {
                continue;
            }

            for group in &self.groups {
                for permutation in group.teams.iter().permutations(2) {
                    let [TeamSlot(team_one, t1_avail), TeamSlot(team_two, t2_avail)] =
                        &permutation[..]
                    else {
                        unreachable!()
                    };

                    let overlap = |x: &Slot| {
                        LossyAvailability::overlap_fast(&x.availability, &slot.availability)
                    };

                    if t1_avail.iter().any(overlap) || t2_avail.iter().any(overlap) {
                        continue;
                    }

                    result.push(Reservation {
                        slot: *slot,
                        game: Some(Game {
                            team_one: *team_one,
                            team_two: *team_two,
                            group_id: group.id(),
                        }),
                    })
                }
            }
        }

        result.to_vec()
    }

    fn make_move(&mut self, mov: &Self::Move) {
        let handle = &mov.game;

        let Some(game) = handle.as_ref() else {
            unreachable!();
        };

        let t1_vec = &mut self.groups[(game.group_id.get() - 1) as usize]
            .get_team(game.team_one.id)
            .1;

        t1_vec.push(mov.slot);

        let t2_vec = &mut self.groups[(game.group_id.get() - 1) as usize]
            .get_team(game.team_two.id)
            .1;

        t2_vec.push(mov.slot);

        self.games.insert(mov.slot, mov.game);
    }
}

impl TranspositionHash for MCTSState {
    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.games.hash(&mut hasher);
        hasher.finish()
    }
}

struct ScheduleEvaluator;

impl Evaluator<SchedulerMCTS> for ScheduleEvaluator {
    type StateEvaluation = i16;

    #[inline(always)]
    fn evaluate_new_state(
        &self,
        state: &MCTSState,
        moves: &Vec<Reservation>,
        _: Option<SearchHandle<SchedulerMCTS>>,
    ) -> (Vec<()>, i16) {
        let mut result = 0;

        let mut busy: BTreeMap<Team, Vec<Slot>> = BTreeMap::new();

        for (slot, game) in &state.games {
            if let Some(game) = game.as_ref() {
                let mut entry = busy.entry(game.team_one).or_default();
                entry.push(*slot);

                entry = busy.entry(game.team_two).or_default();
                entry.push(*slot);

                result += 1;
            } else {
                result -= 10;
            }
        }

        if !busy.is_empty() {
            let frequency_of_distribution = busy.values().map(Vec::len);

            /*
             * Prefer a smaller spread of values
             */
            if let MinMaxResult::MinMax(min, max) = frequency_of_distribution.minmax() {
                let weight: i16 = match max - min {
                    0 => 5,
                    1 => 1,
                    2 => -1,
                    3 => -5,
                    4 => -15,
                    5 => -25,
                    _ => -40,
                };

                result += weight;
            }
        }

        // no allocation
        (vec![(); moves.len()], result)
    }

    fn interpret_evaluation_for_player(&self, evaln: &i16, _player: &()) -> i64 {
        *evaln as i64
    }

    fn evaluate_existing_state(
        &self,
        _: &MCTSState,
        evaln: &i16,
        _: SearchHandle<SchedulerMCTS>,
    ) -> i16 {
        *evaln
    }
}

#[derive(Default)]
struct SchedulerMCTS(usize);

impl SchedulerMCTS {
    pub const fn new(max_playout_length: usize) -> Self {
        Self(max_playout_length)
    }
}

impl MCTS for SchedulerMCTS {
    type State = MCTSState;
    type Eval = ScheduleEvaluator;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ApproxTable<Self>;

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::UseCurrentEvalWhenCycleDetected
    }

    fn max_playout_length(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Output {
    reservations: Vec<Reservation>,
    time_taken: Duration,
    fillage: f32,
}

impl Output {
    pub fn all_booked(&self) -> bool {
        self.fillage == 1.
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut r = self.reservations.iter();

        if let Some(first) = r.next() {
            write!(f, "- {first}")?;
        }

        for reservation in r {
            write!(f, "\n- {reservation}")?;
        }

        write!(
            f,
            "\nBooked {:.2}%; Elapsed {}s",
            self.fillage * 100.,
            self.time_taken.as_secs_f32()
        )
    }
}

pub(crate) fn schedule(state: MCTSState) -> Result<Output> {
    let mut best: Option<Output> = None;

    const RETRIES: u8 = 10;

    for i in 1..=RETRIES {
        let out = schedule_once(state.clone())?;
        if out.all_booked() {
            return Ok(out);
        }

        let percent = out.fillage * 100.;

        log::warn!("Recieved an output that booked {percent:.2}% of all matches, will retry {} more times.", RETRIES - i);

        if let Some(ref best_unwrapped) = best {
            if out.fillage <= best_unwrapped.fillage {
                log::warn!("Current best is {percent:.2}%");
                continue;
            }
        }

        best = Some(out);
    }

    log::error!("Returning a sub-optimal schedule: {best:?}");

    best.ok_or_else(|| unreachable!("the scheduler should have retried"))
}

pub(crate) fn schedule_once(state: MCTSState) -> Result<Output> {
    let total_slots = state.games.len();
    let team_len = state.teams_len();

    let mut mcts = MCTSManager::new(
        state.clone(),
        SchedulerMCTS::new(total_slots),
        ScheduleEvaluator,
        UCTPolicy::new(0.01),
        ApproxTable::new(4096),
    );

    let iterations = if (8..=13).contains(&team_len) {
        (10_000. * (20. * team_len as f32 + 10.).powf(0.335)) as u32
    } else {
        /*
         * Lower rounds need more iterations because the algorithm
         * will have less of a chance of picking the right path.
         *
         * Higher rounds (>14) are not going to be supported as well
         * as lower team counts.
         */
        100_000
    };

    let runners = std::thread::available_parallelism()
        .expect("could not get thread data")
        .get();

    log::info!("Scheduling for {iterations} iterations on {runners} threads.");

    let start = Instant::now();

    mcts.playout_n_parallel(iterations, runners);

    let end = Instant::now();

    let time_taken = end.duration_since(start);

    log::info!("... Done in {:.3}s", time_taken.as_secs_f32());

    let mut result = vec![];

    for m in mcts.principal_variation(total_slots) {
        result.push(m);
    }

    result.sort_by_key(|r| r.slot.availability.0);

    Ok(Output {
        fillage: result.len() as f32 / total_slots as f32,
        reservations: result,
        time_taken,
    })
}

pub fn test() -> Result<()> {
    let mut state = MCTSState::new();

    state.add_time_slots(
        1,
        [
            window!(11/9/2006 from 9:30 to 11:00)?,
            window!(11/9/2006 from 11:30 to 13:00)?,
            window!(12/9/2006 from 13:30 to 15:00)?,
            window!(12/9/2006 from 8:00 to 9:30)?,
            window!(12/9/2006 from 10:00 to 11:30)?,
            window!(13/9/2006 from 12:00 to 13:00)?,
            window!(14/9/2006 from 8:00 to 9:30)?,
            window!(14/9/2006 from 14:00 to 15:30)?,
            window!(14/9/2006 from 16:00 to 17:30)?,
        ],
    );

    state.add_time_slots(
        2,
        [
            window!(11/9/2006 from 9:30 to 11:00)?,
            window!(11/9/2006 from 11:30 to 13:00)?,
            window!(12/9/2006 from 13:30 to 15:00)?,
            window!(12/9/2006 from 8:00 to 9:30)?,
            window!(12/9/2006 from 10:00 to 11:30)?,
            window!(13/9/2006 from 12:00 to 13:00)?,
            window!(14/9/2006 from 8:00 to 9:30)?,
            window!(14/9/2006 from 14:00 to 15:30)?,
            window!(14/9/2006 from 16:00 to 17:30)?,
        ],
    );

    state.add_time_slots(
        3,
        [
            window!(11/10/2006 from 9:30 to 11:00)?,
            window!(11/10/2006 from 11:30 to 13:00)?,
            window!(12/10/2006 from 13:30 to 15:00)?,
            window!(12/10/2006 from 8:00 to 9:30)?,
            window!(12/10/2006 from 10:00 to 11:30)?,
            window!(13/10/2006 from 12:00 to 13:00)?,
            window!(14/10/2006 from 8:00 to 9:30)?,
            window!(14/10/2006 from 14:00 to 15:30)?,
            window!(14/10/2006 from 16:00 to 17:30)?,
        ],
    );

    let mut group_one = PlayableGroup::new(0);

    for i in 0..8 {
        group_one.add_team(i);
    }

    state.add_group(group_one);

    let result = schedule(state)?;

    println!("{result}");

    Ok(())
}