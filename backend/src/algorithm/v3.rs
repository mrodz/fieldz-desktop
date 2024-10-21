use std::{fmt::Debug, sync::{Arc, Mutex}};

use anyhow::Result;
use argmin::core::CostFunction;
use petgraph::prelude::UnGraphMap;
use rand::rngs::SmallRng;

use crate::{
    AvailabilityWindow, CoachConflictLike, FieldLike, Output, PlayableTeamCollection, ScheduledInput, TeamLike
};

use super::v2::PlayableGroup;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Team {
    id: i32,
}

impl TeamLike for Team {
    fn unique_id(&self) -> i32 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSlot {
    field_id: i32,
    window: AvailabilityWindow,
}

pub struct GameScheduleProblem {
	/// # Todo:
	/// Profile whether Arc<RwLock<UnGraphMap>> is more efficient
	team_collisions: UnGraphMap<Team, ()>,
    rng: Arc<Mutex<SmallRng>>,
    groups: Box<[PlayableGroup]>,
    time_slots: Box<[TimeSlot]>,
}

pub struct Game {
    home: Team,
    away: Team,
}

type ParameterVector = Vec<(TimeSlot, Box<[Option<Game>]>)>;

impl CostFunction for GameScheduleProblem {
    type Param = ParameterVector;
    type Output = f64;

    fn cost(&self, param: &Self::Param) -> std::result::Result<Self::Output, anyhow::Error> {
        todo!()
    }
}

pub fn schedule<T, P, F, C>(input: ScheduledInput<T, P, F, C>) -> Result<Output<T, F>>
where
    T: TeamLike + Clone + Debug + PartialEq + Send,
    P: PlayableTeamCollection<Team = T> + Send,
    F: FieldLike + Clone + Debug + PartialEq + Send,
    C: CoachConflictLike + Send,
{
    todo!()
}
