//! Stores all the logic needed to generate a Pre-Schedule Report.
//! This report communicates potential clashes and misuse, alongside
//! the requirements to proceed with schedule creation.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Display,
    num::NonZeroU8,
    ops::AddAssign,
};

use entity::{field, region, team, team_group, team_group_join};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QuerySelect,
    RelationTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    select_time_slot_extension, PreScheduleReportError, TargetExtension, TimeSlotExtension,
    TimeSlotSelectionTypeAggregate,
};

use crate::entity_local_exports::{FieldEntity, TargetEntity, TeamEntity, TeamGroup};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionalUnionU64 {
    Interregional(u64),
    Regional(Vec<(i32, u64)>),
}

impl Display for RegionalUnionU64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interregional(matches) => write!(f, "{matches} matches"),
            Self::Regional(region_based_matches) => {
                let mut iter = region_based_matches.iter();

                let Some((first_region_id, first_match)) = iter.next() else {
                    return Ok(());
                };

                write!(f, "region {first_region_id}: {first_match}")?;

                for (region_id, matches) in iter {
                    write!(f, ", region {region_id}: {matches}")?;
                }

                Ok(())
            }
        }
    }
}

impl RegionalUnionU64 {
    pub const fn default(interregional: bool) -> Self {
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

    pub fn satisfies(&self, predicate: &Self) -> bool {
        match (self, predicate) {
            (Self::Regional(lhs), Self::Regional(predicate)) => {
                'outer: for predicate_item in predicate {
                    for lhs_item in lhs {
                        if lhs_item.0 == predicate_item.0 {
                            if lhs_item.1 < predicate_item.1 {
                                return false;
                            }
                            continue 'outer;
                        }
                    }
                }
                true
            }
            (Self::Interregional(lhs), Self::Interregional(predicate)) => lhs >= predicate,
            (lhs, rhs) => panic!("type mismatch: comparing {lhs:?} to {rhs:?}"),
        }
    }

    pub fn reduce_by_and_ret_overflow(&mut self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Regional(lhs), Self::Regional(other)) => {
                let mut result = Self::Regional(vec![]);

                let mut did_overflow = false;

                'outer: for other_item in other {
                    for lhs_item in lhs.iter_mut() {
                        if lhs_item.0 == other_item.0 {
                            if let Some(new_lhs) = lhs_item.1.checked_sub(other_item.1) {
                                lhs_item.1 = new_lhs;
                            } else {
                                did_overflow = true;
                                result +=
                                    Self::Regional(vec![(lhs_item.0, other_item.1 - lhs_item.1)])
                            }

                            continue 'outer;
                        }
                    }
                }

                if did_overflow {
                    Some(result)
                } else {
                    None
                }
            }
            (Self::Interregional(lhs), Self::Interregional(other)) => {
                if let Some(new_lhs) = lhs.checked_sub(*other) {
                    *lhs = new_lhs;
                    None
                } else {
                    Some(Self::Interregional(*other - *lhs))
                }
            }
            (lhs, rhs) => panic!("type mismatch: reducing {lhs:?} by {rhs:?}"),
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
pub struct SupplyRequireEntry {
    target: TargetExtension,
    required: RegionalUnionU64,
    supplied: RegionalUnionU64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreScheduleReport {
    target_duplicates: Vec<DuplicateEntry>,
    target_has_duplicates: Vec<usize>,
    target_match_count: Vec<SupplyRequireEntry>,
    total_matches_required: u64,
    total_matches_supplied: u64,
    interregional: bool,
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
    /// Build a PreScheduleReport from its components
    /// 
    /// # Parameters
    /// - `target_duplicates`: A list of bundled team & group data to guide the processor
    /// - `all_targets`: A list of all targets and extended data
    /// - `all_time_slots`: A list of all time slots and extended data
    /// - `region_id_from_field_id`: Dependency injection to delegate region lookup responsibility to the caller
    /// - `input`: The input payload given by the client  
    pub fn new(
        target_duplicates: Vec<DuplicateEntry>,
        all_targets: &[TargetExtension],
        all_time_slots: &[TimeSlotExtension],
        region_id_from_field_id: impl Fn(i32) -> i32,
        input: PreScheduleReportInput,
    ) -> Self {
        let target_has_duplicates = target_duplicates
            .iter()
            .filter(|d| d.has_duplicates())
            .flat_map(|d| &d.used_by)
            .map(|target| target.target.id.try_into().unwrap())
            .collect();

        // match tally required for a schedule to populate
        let mut target_required_matches: BTreeMap<&TargetExtension, RegionalUnionU64> =
            BTreeMap::new();

        // match tally to keep track of the space available per field size/type
        let mut field_size_supplied_matches: BTreeMap<i32, RegionalUnionU64> = BTreeMap::new();

        // upsert the matches of the time slot to be indexed by the id of the field size
        for time_slot_ext in all_time_slots {
            let matches_for_this_reservation_type = field_size_supplied_matches
                .entry(time_slot_ext.reservation_type.id)
                .or_insert(RegionalUnionU64::default(input.interregional));

            let this_raw_matches = time_slot_ext.matches_played().try_into().unwrap();

            *matches_for_this_reservation_type += if input.interregional {
                RegionalUnionU64::Interregional(this_raw_matches)
            } else {
                RegionalUnionU64::Regional(vec![(
                    region_id_from_field_id(time_slot_ext.time_slot.field_id),
                    this_raw_matches,
                )])
            };
        }

        // for metadata
        let mut total_matches_required = 0;

        // calculate the number of matches needed to be played with some `n` teams via the choice formula
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
                let require_sum = target_required_matches
                    .entry(target)
                    .or_insert(RegionalUnionU64::default(input.interregional));

                *require_sum += choices.clone();
            }
        }

        // multiply the data collected above (play out of one game tops per combination) by the number of games desired for each combination
        for m in &mut target_required_matches {
            m.1.spread_mul(input.matches_to_play.get() as u64);
        }

        let mut target_supplied_matches: BTreeMap<&TargetExtension, RegionalUnionU64> =
            BTreeMap::new();

        let total_matches_supplied = input.total_matches_supplied.unwrap();

        // populate the amount of matches supplied by the time slots
        for time_slot_ext in all_time_slots {
            for target_ext in dbg!(all_targets) {
                let maybe_type = target_ext.target.maybe_reservation_type;
                let matches_played = u64::try_from(time_slot_ext.matches_played()).unwrap();

                // proceed if the target's and time slot's reservation type match, or no reservation type was specified
                if maybe_type.is_some_and(|id| id != time_slot_ext.reservation_type.id) {
                    continue;
                }

                let will_be_added = if input.interregional {
                    RegionalUnionU64::Interregional(matches_played)
                } else {
                    let region_id = region_id_from_field_id(time_slot_ext.time_slot.field_id);
                    RegionalUnionU64::Regional(vec![(region_id, matches_played)])
                };

                let Some(required_for_this_reservation_type) =
                    field_size_supplied_matches.get_mut(&time_slot_ext.reservation_type.id)
                else {
                    unreachable!("this should have been inserted earlier")
                };

                /*
                 * Keep in mind that targets overlap: `boys u8` and `girls u8` will both play on
                 * the same field size, which must be taken into account. This is done by
                 * decreasing the "availability" of matches indexed by field type below.
                 */
                if let Some(_overflow_data) =
                    required_for_this_reservation_type.reduce_by_and_ret_overflow(&will_be_added)
                {
                    continue;
                }

                let entry = target_supplied_matches
                    .entry(target_ext)
                    .or_insert(RegionalUnionU64::default(input.interregional));

                *entry += will_be_added;
            }
        }

        let mut target_match_count = Vec::with_capacity(target_required_matches.len());

        /*
         * Compute missing parameters and package errors
         */

        let supplied_keys = BTreeSet::from_iter(target_supplied_matches.keys().cloned());
        let required_keys = BTreeSet::from_iter(target_required_matches.keys().cloned());

        let missing_supplied = required_keys.difference(&supplied_keys);
        let missing_required = supplied_keys.difference(&required_keys);

        for missing_target in missing_supplied {
            let union = match target_required_matches.get(*missing_target).unwrap() {
                RegionalUnionU64::Interregional(_) => RegionalUnionU64::Interregional(0),
                RegionalUnionU64::Regional(rid_count) => {
                    RegionalUnionU64::Regional(rid_count.iter().map(|(rid, _)| (*rid, 0)).collect())
                }
            };

            target_supplied_matches.insert(missing_target, union);
        }

        for missing_target in missing_required {
            let union = match target_supplied_matches.get(*missing_target).unwrap() {
                RegionalUnionU64::Interregional(_) => RegionalUnionU64::Interregional(0),
                RegionalUnionU64::Regional(rid_count) => {
                    RegionalUnionU64::Regional(rid_count.iter().map(|(rid, _)| (*rid, 0)).collect())
                }
            };

            target_required_matches.insert(missing_target, union);
        }

        /*
         * Package results
         */

        // zip works because we can assume the maps share the same `&TargetExtension` as keys and the
        // values have equal cardinality and sort because they are `BTreeMaps`
        let result_iterator = target_required_matches
            .into_iter()
            .zip(target_supplied_matches.into_iter());

        for (required, supplied) in result_iterator {
            if required.0.target.id != supplied.0.target.id {
                panic!("{required:?} does not have the same target as {supplied:?}");
            }

            target_match_count.push(SupplyRequireEntry {
                target: required.0.clone(),
                required: required.1,
                supplied: supplied.1,
            })
        }

        Self {
            target_duplicates,
            target_has_duplicates,
            target_match_count,
            total_matches_required: total_matches_required * input.matches_to_play.get() as u64,
            total_matches_supplied,
            interregional: input.interregional,
        }
    }

    /// Build a PreScheduleReport from a dynamic data source
    /// # Performance
    /// This function is computationally expensive and may take significant time to execute.
    /// It is recommended to avoid calling this function frequently in performance-critical code paths.
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

        let all_time_slots: Vec<TimeSlotExtension> = select_time_slot_extension()
            .into_model::<TimeSlotSelectionTypeAggregate>()
            .all(connection)
            .await
            .map(|v| v.into_iter().map(Into::into).collect())
            .map_err(|e| {
                PreScheduleReportError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
            })?;

        if input.total_matches_supplied.is_none() {
            let result: u64 = all_time_slots
                .iter()
                .map(|time_slot_ext| time_slot_ext.matches_played())
                .sum::<i32>()
                .try_into()
                .unwrap();

            input.total_matches_supplied = Some(result);
        };

        let field_to_region = BTreeMap::from_iter(
            FieldEntity::find()
                .select_only()
                .column_as(field::Column::Id, "field_id")
                .column_as(region::Column::Id, "region_id")
                .join(JoinType::Join, field::Relation::Region.def())
                .into_tuple::<(i32, i32)>()
                .all(connection)
                .await
                .map_err(|e| {
                    PreScheduleReportError::DatabaseError(format!("{e} {}:{}", line!(), column!()))
                })?
                .into_iter(),
        );

        Ok(Self::new(
            target_duplicates,
            &all_targets_extended,
            &all_time_slots,
            |field_id| field_to_region[&field_id],
            input,
        ))
    }
}
