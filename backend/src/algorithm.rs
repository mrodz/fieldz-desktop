use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use itertools::Itertools;
use itertools::MinMaxResult;
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
    teams: Vec<(Team, Vec<AvailabilityWindow>)>,
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
        x.teams = teams.into().into_iter().map(|x| (x, vec![])).collect();
        x
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
                        let [(team_one, t1_avail), (team_two, t2_avail)] = &permutation[..] else {
                            unreachable!()
                        };

                        if t1_avail
                            .iter()
                            .any(|x| AvailabilityWindow::overlap_fast(x, &availability_window))
                            || t2_avail
                                .iter()
                                .any(|x| AvailabilityWindow::overlap_fast(x, &availability_window))
                        {
                            continue;
                        }

                        result.push(Reservation {
                            availability_window: availability_window.clone(),
                            game: Arc::new(RwLock::new(Some(Game {
                                team_one: team_one.clone(),
                                team_two: team_two.clone(),
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

        let mut busy: HashMap<Team, Vec<AvailabilityWindow>> = HashMap::new();

        for (availability, game) in &state.games {
            if let Some(game) = game.0.read().unwrap().as_ref() {
                let mut entry = busy.entry(game.team_one.clone()).or_default();
                entry.push(availability.clone());

                entry = busy.entry(game.team_two.clone()).or_default();
                entry.push(availability.clone());

                result += 1;
            } else {
                result -= 1;
            }
        }

        if !busy.is_empty() {
            let games_len = state.games.len();

            let perfect_mean = games_len as f32 / state.teams.len() as f32;

            let frequency_of_distribution = busy.values().map(Vec::len).collect_vec();
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

            if let MinMaxResult::MinMax(min, max) = frequency_of_distribution.iter().minmax() {
                let delta = max - min;

                let std_deviation_spread = delta as f32 / data_std_deviation;

                if std_deviation_spread < 1.0 {
                    result += 3;
                } else if std_deviation_spread < 2.0 {
                    result += 1;
                } else if std_deviation_spread < 3.0 {
                    result -= 1;
                } else if std_deviation_spread < 4.0 {
                    result -= 2;
                } else if std_deviation_spread < 5.0 {
                    result -= 5;
                } else if std_deviation_spread < 6.0 {
                    result -= 10;
                } else {
                    result -= 30;
                }
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
            window!(14/9/2006 from 16:00 to 17:30)?,
        ],
        [
            Team { id: 0 },
            Team { id: 1 },
            Team { id: 2 },
            Team { id: 3 },
            Team { id: 4 },
            Team { id: 5 },
            Team { id: 6 },
        ],
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
    mcts.playout_n_parallel(1_000_000, 40); // 10000 playouts, 4 search threads

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
