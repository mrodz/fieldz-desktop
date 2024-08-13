use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use argmin::{
    core::{CostFunction, Executor, State},
    solver::simulatedannealing::{Anneal, SimulatedAnnealing},
};
use petgraph::prelude::UnGraphMap;
use rand::{
    distributions::Uniform,
    rngs::SmallRng,
    seq::{index::sample, SliceRandom},
    Rng, SeedableRng,
};

use crate::{
    AvailabilityWindow, Booking, BusyTeamQueue, CoachConflictLike, FieldLike, Output,
    PlayableTeamCollection, Reservation, ScheduledInput, TeamLike,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSlot {
    field_id: i32,
    window: AvailabilityWindow,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Team {
    id: i32,
}

impl TeamLike for Team {
    fn unique_id(&self) -> i32 {
        self.id
    }
}

pub type ParameterVector = Vec<(TimeSlot, Option<Team>)>;

pub struct PracticeScheduleProblem {
    team_collisions: UnGraphMap<Team, ()>,
    rng: Arc<Mutex<SmallRng>>,
    teams: Box<[Team]>,
    time_slots: Box<[TimeSlot]>,
}

impl PracticeScheduleProblem {
    /// # Give a hint to the annealer so that early rounds work better
    pub fn seed(&mut self) -> ParameterVector {
        let mut rng = self.rng.lock().unwrap();

        self.teams.shuffle(&mut *rng);

        let mut result: ParameterVector = Vec::new();

        let mut time_slot_indices =
            sample(&mut *rng, self.time_slots.len(), self.time_slots.len()).into_iter();

        for team in self.teams.iter().cycle() {
            let Some(time_slot_index) = time_slot_indices.next() else {
                break;
            };
            let time_slot = &self.time_slots[time_slot_index];
            result.push((time_slot.clone(), Some(*team)))
        }
        // self.time_slots.iter().map(|ts| (ts.clone(), None)).collect_vec()
        result
    }
}

impl CostFunction for PracticeScheduleProblem {
    type Param = ParameterVector;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output> {
        // println!("getting cost...");
        let mut busy_queue = BusyTeamQueue::default();

        let mut empty_matches = 0_f64;
        let mut conflicts = 0_f64;

        for (time_slot, team) in param {
            let Some(team) = team else {
                empty_matches += 1.;
                continue;
            };
            if busy_queue.is_busy(team.id, &time_slot.window) {
                conflicts += 1.;
            }

            busy_queue.add_team(team.id, time_slot.window.clone());

            let related_teams = self.team_collisions.edges(*team);
            // there will realistically never be billions of teams so the cast is safe
            let count_of_related_teams = related_teams
                .filter(|(_src_team, team, _)| busy_queue.is_busy(team.id, &time_slot.window))
                .count() as f64;
            conflicts += count_of_related_teams;
        }

        if conflicts != 0. {
            Ok(conflicts * 2000.)
        } else {
            Ok(empty_matches * 0.5)
        }
    }

    fn parallelize(&self) -> bool {
        true
    }
}

impl Anneal for PracticeScheduleProblem {
    type Param = ParameterVector;
    type Output = ParameterVector;
    type Float = f64;

    fn anneal(&self, param: &Self::Param, extent: Self::Float) -> Result<Self::Output> {
        let mut param_n = param.clone();
        let mut rng = self.rng.lock().unwrap();
        let distr = Uniform::from(0..param.len());

        let mut busy_queue = BusyTeamQueue::default();

        // these indices are for time slots that probably have issues
        let mut target_swap_indices = vec![];
        // these indices are for time slots that can be swapped with a problematic time slot
        let mut free_indices = vec![];

        for (i, (time_slot, practice)) in param.iter().enumerate() {
            let Some(team) = practice else {
                free_indices.push(i);
                continue;
            };

            let related_teams = self.team_collisions.edges(*team);

            // there will realistically never be billions of teams so the cast is safe
            let count_of_related_teams = related_teams
                .filter(|(_src_team, team, _)| busy_queue.is_busy(team.id, &time_slot.window))
                .count();

            if count_of_related_teams != 0 {
                target_swap_indices.push(i);
            }

            if busy_queue.is_busy(team.id, &time_slot.window) {
                target_swap_indices.push(i);
                continue;
            }

            if count_of_related_teams != 0 {
                continue;
            }

            busy_queue.add_team(team.id, time_slot.window.clone());
            free_indices.push(i);
        }

        // Perform modifications to a degree proportional to the current temperature `extent`.
        let operations = extent.floor() as u64 + 1;
        for _ in 0..operations {
            if let Some(problematic_time_slot_index) =
                target_swap_indices.choose(&mut *rng).cloned()
            {
                if let Some(ok_index) = free_indices.choose(&mut *rng).cloned() {
                    let ok_1 = param_n[ok_index].1;
                    param_n[ok_index].1 = param_n[problematic_time_slot_index].1;
                    param_n[problematic_time_slot_index].1 = ok_1;

                    free_indices.retain(|index| *index != ok_index);
                    target_swap_indices.retain(|index| *index != problematic_time_slot_index);

                    continue;
                }

                if param_n.len() - target_swap_indices.len() <= 1 {
                    break;
                }

                let index = loop {
                    let maybe_index = rng.sample(distr);
                    if problematic_time_slot_index != maybe_index
                        && !target_swap_indices.contains(&maybe_index)
                    {
                        break maybe_index;
                    }
                };

                let ok_1 = param_n[index].1;
                param_n[index].1 = param_n[problematic_time_slot_index].1;
                param_n[problematic_time_slot_index].1 = ok_1;

                target_swap_indices.retain(|index| *index != problematic_time_slot_index);

                continue;
            }
        }

        Ok(param_n)
    }
}

/// [`PlayableTeamCollection`] is ignored as practices are scoped by field type, not by arbitrary collections.
pub fn schedule<T, P, F, C>(input: ScheduledInput<T, P, F, C>) -> Result<Output<T, F>>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    let mut team_collisions = UnGraphMap::new();

    for coach_conflict in input.coach_conflicts() {
        let teams = coach_conflict.teams(); // lifetime purposes
        let teams = teams.as_ref();
        for (i, team_as_node) in teams.iter().enumerate() {
            let team_as_node = Team {
                id: team_as_node.unique_id(),
            };

            let node = team_collisions.add_node(team_as_node);

            for (j, team) in teams.iter().enumerate() {
                if j == i {
                    continue;
                }

                let team = Team {
                    id: team.unique_id(),
                };

                team_collisions.add_edge(node, team, ());
            }
        }
    }

    let mut teams = Vec::with_capacity(input.teams_len());
    let mut teams_lookup = BTreeMap::new();

    for team_collection in input.team_groups() {
        let teams_ghost = team_collection.teams(); // for lifetime purposes
        for team in teams_ghost.as_ref() {
            teams.push(Team {
                id: team.unique_id(),
            });
            teams_lookup.insert(team.unique_id(), team.clone());
        }
    }

    let mut time_slots = Vec::new();

    let mut fields = BTreeMap::new();

    for field in input.fields() {
        fields.insert(field.unique_id(), field.clone());
    }

    for field in input.fields() {
        for ((start, end), _) in field.time_slots().as_ref() {
            time_slots.push(TimeSlot {
                field_id: field.unique_id(),
                window: AvailabilityWindow::new_unix(*start, *end)?,
            });
        }
    }

    let teams_len = teams.len();

    let mut problem = PracticeScheduleProblem {
        team_collisions,
        rng: Arc::new(Mutex::new(SmallRng::from_entropy())),
        teams: teams.into(),
        time_slots: time_slots.clone().into(),
    };

    let init = problem.seed();

    let time_slots_len = time_slots.len() as f64;

    let max_iters = if cfg!(feature = "env_sa_max_iters") {
        option_env!("SA_MAX_ITERS")
            .or(std::env::var("SA_MAX_ITERS").ok().as_deref())
            .and_then(|env_var| env_var.parse::<u64>().ok())
            .unwrap_or(100)
    } else {
        let time_complexity = time_slots_len.log2() * time_slots_len * teams_len as f64;
        time_complexity.floor() as u64 * 100
    };

    /*
     * A temperature too high relative to the differences in solution quality (points to anneal) can
     * result in nearly all proposed moves being accepted, regardless of whether they improve the
     * solution or not. This can lead to inefficient exploration.
     */
    let temperature = time_slots_len.clamp(1.0, time_slots_len * 1.2);

    println!("Solving practices (i={max_iters}, temperature={temperature} degrees)");

    let solver = SimulatedAnnealing::new(temperature)?
        .with_reannealing_accepted(3 * max_iters / 2)
        .with_reannealing_best(4 * max_iters / 5);

    let res = Executor::new(problem, solver)
        .configure(|state| state.param(init).max_iters(max_iters).target_cost(0.0))
        .run()?;

    let Some(winner) = res.state().get_best_param() else {
        return Ok(Output {
            time_slots: time_slots
                .into_iter()
                .map(|ts| Reservation {
                    availability: ts.window,
                    booking: crate::Booking::Empty,
                    field: fields[&ts.field_id].clone(),
                })
                .collect(),
            unique_id: input.unique_id,
        });
    };

    Ok(Output {
        time_slots: winner
            .iter()
            .map(|(ts, winner)| Reservation {
                availability: ts.window.clone(),
                booking: winner.map_or_else(
                    || Booking::Empty,
                    |team| Booking::Practice(teams_lookup[&team.id].clone()),
                ),
                field: fields[&ts.field_id].clone(),
            })
            .collect(),
        unique_id: input.unique_id,
    })
}
