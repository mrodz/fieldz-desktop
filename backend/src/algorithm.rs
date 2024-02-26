use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use itertools::Itertools;
use mcts::transposition_table::*;
use mcts::tree_policy::*;
use mcts::*;

use crate::window;
use crate::AvailabilityWindow;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Team {
    id: u8,
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
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} v {}", self.team_one, self.team_two)
    }
}

#[derive(Clone, Debug)]
struct Reservation {
    availability_window: AvailabilityWindow,
    game: Arc<RwLock<Option<Game>>>,
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.availability_window == other.availability_window
            && self.game.read().unwrap().eq(&other.game.read().unwrap())
    }
}

impl Eq for Reservation {}

impl Hash for Reservation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.availability_window.hash(state);
        self.game.read().unwrap().hash(state);
    }
}

impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.availability_window)?;

        if let Some(game) = self.game.read().unwrap().as_ref() {
            write!(f, " @ {game}")
        } else {
            write!(f, " <wasted>")
        }
    }
}

// A really simple game. There's one player and one number. In each move the player can
// increase or decrease the number. The player's score is the number.
// The game ends when the number reaches 100.
//
// The best strategy is to increase the number at every step.

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct MCTSState {
    games: BTreeMap<AvailabilityWindow, MutableGame>,
    teams: Vec<Team>,
}

impl<T> From<T> for MCTSState
where
    T: AsRef<[AvailabilityWindow]>,
{
    fn from(value: T) -> Self {
        let mut games = BTreeMap::new();
        for time_slot in value.as_ref() {
            games.insert(time_slot.clone(), MutableGame(Arc::default()));
        }
        MCTSState {
            games,
            teams: vec![],
        }
    }
}

enum Conflicts {
    Weighted {
        weighted_collisions: i64,
        weighted_unused_matches: i64,
        weighted_maldistributions: f32,
    },
    Sentinel(i64),
}

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

impl MCTSState {
    pub fn new(
        availability_windows: impl AsRef<[AvailabilityWindow]>,
        teams: impl Into<Vec<Team>>,
    ) -> Self {
        let mut x = Self::from(availability_windows);
        x.teams = teams.into();

        x
    }

    pub fn conflicts(&self, use_sentinel: bool) -> Conflicts {
        let mut busy: HashMap<Team, Vec<AvailabilityWindow>> = HashMap::new();

        let mut unused_matches: i64 = 0;

        for (availability, game) in &self.games {
            if let Some(game) = game.0.read().unwrap().as_ref() {
                let mut entry = busy.entry(game.team_one.clone()).or_default();
                entry.push(availability.clone());

                entry = busy.entry(game.team_two.clone()).or_default();
                entry.push(availability.clone());
            } else {
                unused_matches += 1;
            }
        }

        let games_len = self.games.len();

        let weighted_maldistributions = 'dist: {
            if !busy.is_empty() {
                if busy.len() < self.teams.len() {
                    if use_sentinel {
                        return Conflicts::Sentinel(-1);
                    }

                    break 'dist 10000000f32;
                }

                let perfect_mean = games_len as f32 / self.teams.len() as f32;

                let frequency_of_distribution = busy.values().map(Vec::len).collect_vec();
                let data_std_deviation = std_deviation(perfect_mean, &frequency_of_distribution)
                    .expect("could not get standard deviation");

                let mut weighted_maldistributions = 1f32;

                for frequency in frequency_of_distribution {
                    // if any team plays 20% more games, that is bad
                    let ratio = frequency as f32 / data_std_deviation;

                    // higher means earlier in the search; lower means later
                    let composition_of_unused = unused_matches as f32 / games_len as f32;

                    if ratio > 1.2 && use_sentinel {
                        return Conflicts::Sentinel(-1);
                    }

                    weighted_maldistributions *=
                        ratio.powf(1.0 / composition_of_unused);
                }

                weighted_maldistributions
            } else {
                0f32
            }
        };

        // threshold = 5% unused
        let weighted_unused_matches =
            (unused_matches as f32 / games_len as f32).powf(6.5) as i64;

        let mut weighted_collisions = 0;

        for windows in busy.values_mut() {
            windows.sort_by_key(|x| x.start);

            // let mut inst_clashes = 0;

            for xy in windows.windows(2) {
                let cond = xy[0].end >= xy[1].start;
                println!("{cond}");
                if cond {
                    weighted_collisions += 10000000i64;
                }
            }
        }

        if use_sentinel && weighted_collisions != 0 {
            return Conflicts::Sentinel(-1);
        }

        if use_sentinel {
            Conflicts::Sentinel(1)
        } else {
            Conflicts::Weighted {
                weighted_collisions,
                weighted_unused_matches,
                weighted_maldistributions,
            }
        }
    }
}

impl GameState for MCTSState {
    type Move = Reservation;
    type Player = ();
    type MoveList = Vec<Reservation>;

    fn current_player(&self) -> Self::Player {
        ()
    }

    fn available_moves(&self) -> Vec<Reservation> {
        // let x = self.0;
        let mut x = self.games.clone();

        x.retain(|_, game| game.is_none());

        if x.len() == 0 {
            vec![]
        } else {
            x.into_iter()
                .flat_map(|(availability_window, _)| {
                    let mut result = vec![];
                    for permutation in self.teams.iter().permutations(2)
                    /*.unique_by(|xy| {
                        if xy[0].id > xy[1].id {
                            (xy[0], xy[1])
                        } else {
                            (xy[1], xy[0])
                        }
                    }) */
                    {
                        let [team_one, team_two] = &permutation[..] else {
                            unreachable!()
                        };

                        result.push(Reservation {
                            availability_window: availability_window.clone(),
                            game: Arc::new(RwLock::new(Some(Game {
                                team_one: (*team_one).clone(),
                                team_two: (*team_two).clone(),
                            }))),
                        })
                    }

                    result
                })
                .collect()
        }
    }

    fn make_move(&mut self, mov: &Self::Move) {
        self.games.insert(
            mov.availability_window.clone(),
            MutableGame(mov.game.clone()),
        );
        // self.games.insert(mov.clone());
    }
}

impl TranspositionHash for MCTSState {
    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.games.hash(&mut hasher);
        hasher.finish()
    }
}

struct MyEvaluator;

impl Evaluator<SchedulerMCTS> for MyEvaluator {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        state: &MCTSState,
        moves: &Vec<Reservation>,
        _: Option<SearchHandle<SchedulerMCTS>>,
    ) -> (Vec<()>, i64) {
        let mut result = 0;

        match state.conflicts(false) {
            Conflicts::Sentinel(val) => {
                result = val;
            }
            Conflicts::Weighted { weighted_collisions, weighted_unused_matches, weighted_maldistributions } => {
                result -= dbg!(weighted_collisions);
                result -= dbg!(weighted_unused_matches);
                result -= dbg!(weighted_maldistributions) as i64 * 30;        
            }
        }

        
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
    type Eval = MyEvaluator;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ApproxTable<Self>;

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::UseCurrentEvalWhenCycleDetected
    }
}

pub fn test() -> Result<()> {
    let game = MCTSState::new(
        [
            window!(11/9/2006 from 9:30 to 11:00)?,
            window!(11/9/2006 from 11:30 to 13:00)?,
            window!(12/9/2006 from 13:30 to 15:00)?,
            window!(12/9/2006 from 8:00 to 9:30)?,
            window!(12/9/2006 from 10:00 to 11:30)?,
            window!(13/9/2006 from 12:00 to 13:00)?,
            window!(14/9/2006 from 8:00 to 9:30)?,
            window!(14/9/2006 from 14:00 to 15:30)?,
        ],
        [Team { id: 0 }, Team { id: 1 }, Team { id: 2 }],
    );

    let total_slots = game.games.len();
    let mut teams_summary: HashMap<Team, u8> = HashMap::with_capacity(game.teams.len());

    let mut mcts = MCTSManager::new(
        game.clone(),
        SchedulerMCTS,
        MyEvaluator,
        UCTPolicy::new(0.5),
        ApproxTable::new(1024),
    );
    mcts.playout_n_parallel(20_000, 8); // 10000 playouts, 4 search threads

    let mut game_count = 0;

    for m in mcts.principal_variation(50) {
        let guard = m.game.read().unwrap();

        let Some(game) = guard.as_ref() else {
            unreachable!()
        };

        println!("{m}");

        let mut c = teams_summary.entry(game.team_one.clone()).or_default();
        *c += 1;

        c = teams_summary.entry(game.team_two.clone()).or_default();
        *c += 1;

        game_count += 1;
    }

    println!("\n{game_count}/{total_slots} slots filled");

    for (team, games_played) in &teams_summary {
        println!("\t- {team} played {games_played} games");
    }

    teams_summary.clear();

    Ok(())
}
