//! Stores all the logic needed to generate a Pre-Schedule Report.
//! This report communicates potential clashes and misuse, alongside
//! the requirements to proceed with schedule creation.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    num::NonZeroU8,
    ops::AddAssign,
};

use entity::{team, team_group, team_group_join};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QuerySelect,
    RelationTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    select_time_slot_extension, PreScheduleReportError, TargetExtension, TimeSlotExtension,
    TimeSlotSelectionTypeAggregate,
};

use crate::entity_local_exports::{TargetEntity, TeamEntity, TeamGroup};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionalUnionU64 {
    Interregional(u64),
    Regional(Vec<(i32, u64)>),
}

impl RegionalUnionU64 {
    pub fn default(interregional: bool) -> Self {
        if interregional {
            Self::Interregional(0)
        } else {
            Self::Regional(vec![])
        }
    }

    pub fn fold_from(iterator: impl Iterator<Item = (i32, u64)>) -> Self {
        let mut result: Vec<(i32, u64)> = vec![];

        'outer: for pair in iterator {
            for (pre_rid, pre_c) in &mut result {
                if *pre_rid == pair.0 {
                    *pre_c += pair.1;
                    continue 'outer;
                }
            }
            result.push(pair);
        }

        Self::Regional(result)
    }

    pub fn sum_total(&self) -> u64 {
        match self {
            Self::Interregional(result) => *result,
            Self::Regional(many_results) => many_results.iter().fold(0, |r, (_, c)| r + c),
        }
    }

    pub fn spread_mul(&mut self, rhs: u64) {
        match self {
            Self::Interregional(c) => *c *= rhs,
            Self::Regional(many_c) => many_c.iter_mut().for_each(|c| c.1 *= rhs),
        }
    }
}

impl AddAssign for RegionalUnionU64 {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Self::Regional(lhs), Self::Regional(rhs)) => {
                'outer: for rhs_item in rhs {
                    for lhs_item in lhs.iter_mut() {
                        if lhs_item.0 == rhs_item.0 {
                            lhs_item.1 += rhs_item.1;
                            continue 'outer;
                        }
                    }
                    lhs.push(rhs_item)
                }
            }
            (Self::Interregional(lhs), Self::Interregional(rhs)) => {
                *lhs = rhs;
            }
            (lhs, rhs) => panic!("type mismatch: adding {lhs:?} to {rhs:?}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateEntry {
    team_groups: Vec<TeamGroup>,
    used_by: Vec<TargetExtension>,
    teams_with_group_set: RegionalUnionU64,
}

impl DuplicateEntry {
    pub fn has_duplicates(&self) -> bool {
        self.used_by.len() > 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreScheduleReport {
    target_duplicates: Vec<DuplicateEntry>,
    target_has_duplicates: Vec<usize>,
    target_required_matches: Vec<(TargetExtension, RegionalUnionU64)>,
    total_matches_required: u64,
    total_matches_supplied: u64,
}

fn ncr(n: u64, r: u64) -> u64 {
    fn factorial(num: u64) -> u64 {
        let mut f = 1;

        for i in 1..=num {
            f *= i;
        }

        f
    }

    if r > n {
        0
    } else {
        factorial(n) / (factorial(r) * factorial(n - r))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreScheduleReportInput {
    matches_to_play: NonZeroU8,
    interregional: bool,
    #[serde(skip)]
    total_matches_supplied: Option<u64>,
}

impl PreScheduleReport {
    pub fn new(target_duplicates: Vec<DuplicateEntry>, input: PreScheduleReportInput) -> Self {
        let target_has_duplicates = target_duplicates
            .iter()
            .filter(|d| d.has_duplicates())
            .flat_map(|d| &d.used_by)
            .map(|target| target.target.id.try_into().unwrap())
            .collect();

        let mut target_required_matches: BTreeMap<TargetExtension, RegionalUnionU64> =
            BTreeMap::new();

        let mut total_matches_required = 0;

        for entry in &target_duplicates {
            let choices = match &entry.teams_with_group_set {
                RegionalUnionU64::Interregional(teams_with_group_set) => {
                    RegionalUnionU64::Interregional(ncr(*teams_with_group_set, 2))
                }
                RegionalUnionU64::Regional(teams_with_group_per_region) => {
                    RegionalUnionU64::fold_from(
                        teams_with_group_per_region
                            .iter()
                            .map(|(region_id, count)| (*region_id, ncr(*count, 2))),
                    )
                }
            };

            total_matches_required += choices.sum_total();
            for target in &entry.used_by {
                let sum = target_required_matches
                    .entry(target.clone())
                    .or_insert(RegionalUnionU64::default(input.interregional));

                *sum += choices.clone();
            }
        }

        for m in &mut target_required_matches {
            m.1.spread_mul(input.matches_to_play.get() as u64);
        }

        // let target_required_matches = if input.interregional {

        // }

        Self {
            target_duplicates,
            target_has_duplicates,
            target_required_matches: target_required_matches.into_iter().collect(),
            total_matches_required: total_matches_required * input.matches_to_play.get() as u64,
            total_matches_supplied: input.total_matches_supplied.unwrap(),
        }
    }

    pub async fn create<C>(
        connection: &C,
        mut input: PreScheduleReportInput,
    ) -> Result<Self, PreScheduleReportError>
    where
        C: ConnectionTrait,
    {
        let all_targets = TargetEntity::find().all(connection).await.map_err(|e| {
            PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
        })?;

        let all_targets_extended = TargetExtension::many_new(&all_targets, connection)
            .await
            .map_err(|e| {
                PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
            })?;

        let mut collision_map: BTreeMap<BTreeSet<&TeamGroup>, Vec<&TargetExtension>> =
            BTreeMap::new();

        for target in &all_targets_extended {
            let set_of_groups = BTreeSet::from_iter(&target.groups);
            let entry = collision_map.entry(set_of_groups).or_default();
            entry.push(target);
        }

        let mut target_duplicates = Vec::with_capacity(collision_map.len());

        for (groups, targets) in collision_map {
            let group_ids = groups.iter().map(|g| g.id).collect::<Vec<_>>();
            let groups_len = groups.len();
            let team_groups = groups.into_iter().cloned().collect();
            let used_by = targets.into_iter().cloned().collect();

            let query = TeamEntity::find()
                .join(JoinType::LeftJoin, team::Relation::TeamGroupJoin.def())
                .join(
                    JoinType::LeftJoin,
                    team_group_join::Relation::TeamGroup.def(),
                )
                .filter(team_group::Column::Id.is_in(group_ids))
                .group_by(team::Column::Id)
                .having(
                    team_group::Column::Id
                        .into_expr()
                        .count_distinct()
                        .eq(i32::try_from(groups_len).unwrap()),
                );

            if input.interregional {
                let teams_with_group_set = query.count(connection).await.map_err(|e| {
                    PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                })?;

                target_duplicates.push(DuplicateEntry {
                    team_groups,
                    used_by,
                    teams_with_group_set: RegionalUnionU64::Interregional(teams_with_group_set),
                });
            } else {
                let mut ordering = HashMap::new();
                let all_teams = query.all(connection).await.map_err(|e| {
                    PreScheduleReportError::DatabaseError(format!("{}:{} {e}", file!(), line!()))
                })?;

                for team in all_teams {
                    let cnt = ordering.entry(team.region_owner).or_default();
                    *cnt += 1_u64;
                }

                target_duplicates.push(DuplicateEntry {
                    team_groups,
                    used_by,
                    teams_with_group_set: RegionalUnionU64::Regional(
                        ordering.into_iter().collect::<Vec<_>>(),
                    ),
                });
            }
        }

        if input.total_matches_supplied.is_none() {
            let all_time_slots: Vec<TimeSlotExtension> = select_time_slot_extension()
                .into_model::<TimeSlotSelectionTypeAggregate>()
                .all(connection)
                .await
                .map(|v| v.into_iter().map(Into::into).collect())
                .map_err(|e| {
                    PreScheduleReportError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?;

            let result: u64 = all_time_slots
                .into_iter()
                .map(|time_slot_ext| time_slot_ext.matches_played())
                .sum::<i32>()
                .try_into()
                .unwrap();

            input.total_matches_supplied = Some(result);
        };

        Ok(Self::new(target_duplicates, input))
    }
}
