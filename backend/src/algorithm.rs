use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

use anyhow::Result;
use itertools::Itertools;
use itertools::MinMaxResult;
use mcts::transposition_table::*;
use mcts::tree_policy::*;
use mcts::*;

use crate::window;
use crate::AvailabilityWindow;

type TeamId = u8;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Team {
    id: TeamId,
}

impl Team {
    fn new(id: TeamId) -> Self {
        Self { id }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct PlayableGroup {
    teams: Vec<TeamSlot>,
    external_id: Option<usize>,
    index_start: usize,
}

impl PlayableGroup {
    pub const fn new(index_start: usize) -> Self {
        Self {
            teams: vec![],
            external_id: None,
            index_start,
        }
    }

    pub fn get_team(&mut self, id: TeamId) -> &mut TeamSlot {
        &mut self.teams[id as usize - self.index_start]
    }

    pub fn add_team(&mut self, id: TeamId) {
        self.teams.push(TeamSlot(Team::new(id), vec![]));
    }

    pub fn set_index(&mut self, external_id: usize) {
        assert!(
            self.external_id.replace(external_id).is_none(),
            "ID was already set"
        );
    }

    #[inline(always)]
    pub fn id(&self) -> usize {
        // unsafe {
        self.external_id.unwrap()
        // }
    }
}

impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Team {}", self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Game {
    team_one: Team,
    team_two: Team,
    group_id: usize,
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} v {}", self.team_one, self.team_two)
    }
}

#[derive(Clone, Debug)]
struct Reservation {
    slot: Slot,
    game: Arc<RwLock<Option<Game>>>,
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.game.read().unwrap().eq(&other.game.read().unwrap())
    }
}

impl Eq for Reservation {}

impl Hash for Reservation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.slot.hash(state);
        self.game.read().unwrap().hash(state);
    }
}

impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slot)?;

        if let Some(game) = self.game.read().unwrap().as_ref() {
            write!(f, " @ {game}")
        } else {
            write!(f, " <wasted>")
        }
    }
}

#[derive(Clone, Debug, Default)]
struct MutableGame(Arc<RwLock<Option<Game>>>);

impl MutableGame {
    pub fn is_none(&self) -> bool {
        self.0.read().unwrap().is_none()
    }
}

impl Hash for MutableGame {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.read().unwrap().hash(state);
    }
}

impl PartialEq for MutableGame {
    fn eq(&self, other: &Self) -> bool {
        self.0.read().unwrap().eq(&other.0.read().unwrap())
    }
}

impl Eq for MutableGame {}

impl From<MutableGame> for Arc<RwLock<Option<Game>>> {
    fn from(value: MutableGame) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Slot {
    field_id: u8,
    availability: AvailabilityWindow,
}

impl Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[f{}] {}", self.field_id, self.availability)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TeamSlot(Team, Vec<Slot>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MCTSState {
    games: BTreeMap<Slot, MutableGame>,
    groups: Vec<PlayableGroup>,
    teams_len: usize,
}

impl MCTSState {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_group(&mut self, mut playable_group: PlayableGroup) {
        playable_group.set_index(self.groups.len());
        self.teams_len += playable_group.teams.len();
        self.groups.push(playable_group);
    }

    pub fn add_time_slots(&mut self, field_id: u8, time_slots: impl AsRef<[AvailabilityWindow]>) {
        for time_slot in time_slots.as_ref() {
            assert!(
                self.games
                    .insert(
                        Slot {
                            field_id,
                            availability: time_slot.clone()
                        },
                        MutableGame::default()
                    )
                    .is_none(),
                "{time_slot} at field {field_id} was already set"
            );
        }
    }

    pub const fn teams_len(&self) -> usize {
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
        let mut result = vec![];

        for (slot, game) in &self.games {
            if !game.is_none() {
                continue;
            }

            for group in &self.groups {
                for permutation in group.teams.iter().permutations(2) {
                    let [TeamSlot(team_one, t1_avail), TeamSlot(team_two, t2_avail)] =
                        &permutation[..]
                    else {
                        unreachable!()
                    };

                    if t1_avail.iter().any(|x| {
                        AvailabilityWindow::overlap_fast(&x.availability, &slot.availability)
                    }) || t2_avail.iter().any(|x| {
                        AvailabilityWindow::overlap_fast(&x.availability, &slot.availability)
                    }) {
                        continue;
                    }

                    result.push(Reservation {
                        slot: slot.clone(),
                        game: Arc::new(RwLock::new(Some(Game {
                            team_one: *team_one,
                            team_two: *team_two,
                            group_id: group.id(),
                        }))),
                    })
                }
            }
        }

        result
    }

    fn make_move(&mut self, mov: &Self::Move) {
        let handle = mov.game.read().unwrap();

        let Some(game) = handle.as_ref() else {
            unreachable!();
        };

        let t1_vec = &mut self.groups[game.group_id].get_team(game.team_one.id).1;

        t1_vec.push(mov.slot.clone());

        let t2_vec = &mut self.groups[game.group_id].get_team(game.team_two.id).1;

        t2_vec.push(mov.slot.clone());

        self.games
            .insert(mov.slot.clone(), MutableGame(mov.game.clone()));
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
    type StateEvaluation = i64;

    #[inline(always)]
    fn evaluate_new_state(
        &self,
        state: &MCTSState,
        moves: &Vec<Reservation>,
        _: Option<SearchHandle<SchedulerMCTS>>,
    ) -> (Vec<()>, i64) {
        let mut result = 0;

        let mut busy: HashMap<Team, Vec<Slot>> = HashMap::new();

        for (slot, game) in &state.games {
            if let Some(game) = game.0.read().unwrap().as_ref() {
                let mut entry = busy.entry(game.team_one).or_default();
                entry.push(slot.clone());

                entry = busy.entry(game.team_two).or_default();
                entry.push(slot.clone());

                result += 1;
            } else {
                result -= 1;
            }
        }

        if !busy.is_empty() {
            let games_len = state.games.len();

            let perfect_mean = games_len as f32 / state.groups.len() as f32;

            let frequency_of_distribution = busy
                .values()
                .map(Vec::len)
                .chain(std::iter::repeat(0).take(state.teams_len() - busy.len()))
                .collect_vec();

            /*
             * This is behind a feature flag because it slows down the ranking
             * phase 5-10x, and doesn't have a major impact on seeding.
             */
            #[cfg(feature = "std_deviation_gate")]
            {
                fn std_deviation(mean: f32, data: &[usize]) -> Option<f32> {
                    if data.len() > 0 {
                        let variance = data
                            .iter()
                            .map(|value| {
                                let diff = mean - (*value as f32);
                                diff * diff
                            })
                            .sum::<f32>()
                            / data.len() as f32;

                        Some(variance.sqrt())
                    } else {
                        None
                    }
                }

                let data_std_deviation = std_deviation(perfect_mean, &frequency_of_distribution)
                    .expect("could not get standard deviation");

                let diff_std_dev_mean = (data_std_deviation - perfect_mean).abs();

                if diff_std_dev_mean < 0.2 {
                    result += 5;
                } else if diff_std_dev_mean < 0.4 {
                    result += 3;
                } else if diff_std_dev_mean < 0.5 {
                    result += 1;
                } else if diff_std_dev_mean < 0.6 {
                    result += 0;
                } else if diff_std_dev_mean < 0.8 {
                    result -= 4;
                } else if diff_std_dev_mean < 1.0 {
                    result -= 7;
                } else if diff_std_dev_mean < 1.2 {
                    result -= 12;
                } else {
                    result -= 30;
                }
            }

            /*
             * Prefer a smaller spread of values
             */
            if let MinMaxResult::MinMax(min, max) = frequency_of_distribution.iter().minmax() {
                match max - min {
                    0 => result += 5,
                    1 => result += 3,
                    2 => result += 0,
                    3 => result -= 3,
                    4 => result -= 8,
                    5 => result -= 15,
                    _ => result -= 30,
                }
            }

            /*
             * Prefer a smaller variance
             */
            let variance = frequency_of_distribution
                .iter()
                .map(|number| (*number as f32 - perfect_mean).powi(2))
                .sum::<f32>()
                / frequency_of_distribution.len() as f32;

            if variance < 0.3 {
                result += 7;
            } else if variance < 0.4 {
                result += 3;
            } else if variance < 0.5 {
                result += 1;
            } else if variance < 0.7 {
                result += 0;
            } else if variance < 1.0 {
                result -= 3;
            } else if variance < 1.3 {
                result -= 8;
            } else if variance < 1.8 {
                result -= 20;
            } else if variance < 6.0 {
                result -= 50;
            } else {
                result -= 100;
            }
        }

        // no allocation
        (vec![(); moves.len()], result)
    }

    fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
        *evaln
    }

    fn evaluate_existing_state(
        &self,
        _: &MCTSState,
        evaln: &i64,
        _: SearchHandle<SchedulerMCTS>,
    ) -> i64 {
        *evaln
    }
}

#[derive(Default)]
struct SchedulerMCTS;

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

    let mut group_one = PlayableGroup::new(0);

    group_one.add_team(0);
    group_one.add_team(1);
    group_one.add_team(2);
    group_one.add_team(3);
    group_one.add_team(4);

    state.add_group(group_one);

    let mut group_two = PlayableGroup::new(5);

    group_two.add_team(5);
    group_two.add_team(6);
    group_two.add_team(7);
    group_two.add_team(8);
    group_two.add_team(9);
    // group_two.add_team(10);
    // group_two.add_team(11);
    // group_two.add_team(12);

    state.add_group(group_two);

    let total_slots = state.games.len();
    let mut teams_summary: HashMap<Team, u8> = HashMap::with_capacity(state.groups.len());

    let mut mcts = MCTSManager::new(
        state.clone(),
        SchedulerMCTS,
        ScheduleEvaluator,
        UCTPolicy::new(0.01),
        ApproxTable::new(4096),
    );

    let iterations = 1_000_000;
    let runners = std::thread::available_parallelism()
        .expect("could not get thread data")
        .get()
        * 3;

    println!("Scheduling for {iterations} iterations on {runners} threads.");

    let start = Instant::now();

    mcts.playout_n_parallel(iterations, runners);

    let end = Instant::now();

    println!(
        "... Done in {:.3}s\n",
        end.duration_since(start).as_secs_f32()
    );

    let mut game_count = 0;

    let mut result = vec![];

    for m in mcts.principal_variation(total_slots) {
        {
            let guard = m.game.read().unwrap();

            let Some(game) = guard.as_ref() else {
                unreachable!()
            };

            let mut c = teams_summary.entry(game.team_one).or_default();
            *c += 1;

            c = teams_summary.entry(game.team_two).or_default();
            *c += 1;

            game_count += 1;
        }

        result.push(m);
    }

    result.sort_by_key(|r| r.slot.availability.start);

    for reservation in result {
        println!("{reservation}");
    }

    println!("\n{game_count}/{total_slots} slots filled");

    for (team, games_played) in &teams_summary {
        println!("\t- {team} played {games_played} games");
    }

    Ok(())
}
