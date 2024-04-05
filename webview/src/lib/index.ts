export interface Region {
	id: number;
	title: string;
}

export interface CreateRegionInput {
	title: string;
}

export interface Field {
	id: number;
	name: string;
	region_owner: number;
}

export interface CreateFieldInput {
	name: string;
	region_id: number;
}

export interface TeamGroup {
	id: number;
	name: string;
	usages: number;
}

export interface TeamExtension {
	team: Team;
	tags: TeamGroup[];
}

export interface Team {
	id: number;
	name: string;
	region_owner: number;
}

export interface CreateTeamInput {
	name: string;
	region_id: number;
	tags: string[];
}

export interface TeamGroup {
	id: number;
	name: string;
}

export interface TimeSlot {
	id: number;
	field_id: number;
	start: string;
	end: string;
}

export interface TimeSlotExtension {
	time_slot: TimeSlot;
	reservation_type: ReservationType;
	custom_matches?: number;
}

export interface CreateTimeSlotInput {
	field_id: number;
	reservation_type_id: number;
	start: number;
	end: number;
}

/**
 * Reference: {@link https://github.com/vkurko/calendar?tab=readme-ov-file#event-object}
 */
export interface CalendarEvent {
	id: string;
	resources: unknown[];
	allDay: boolean;
	start: Date;
	end: Date;
	title?: string | { html: string } | { domNodes: Node[] };
	editable?: boolean;
	startEditable?: boolean;
	durationEditable?: boolean;
	display: 'auto' | 'background';
	backgroundColor?: string;
	eventTextColor?: string;
	extendedProps?: any;
}

export interface MoveTimeSlotInput {
	field_id: number;
	id: number;
	new_start: number;
	new_end: number;
}

export interface ListReservationsBetweenInput {
	start: number;
	end: number;
}

const CALENDAR_TIME_SLOT_COLORS = [
	'#cf625b',
	'#b8cf5b',
	'#5b9fba',
	'#d136c9',
	'#d1aa36',
	'#86a86c',
	'#a134eb',
	'#c2729d',
	'#12a108'
] as const;

function colorForTimeSlot(input: TimeSlot): string {
	return CALENDAR_TIME_SLOT_COLORS[input.field_id % CALENDAR_TIME_SLOT_COLORS.length];
}

export function randomCalendarColor(): typeof CALENDAR_TIME_SLOT_COLORS extends readonly (infer HexCode)[]
	? HexCode
	: never {
	return CALENDAR_TIME_SLOT_COLORS[Math.floor(Math.random() * CALENDAR_TIME_SLOT_COLORS.length)];
}

export function eventFromTimeSlot(input: TimeSlotExtension, title?: string): CalendarEvent {
	return {
		allDay: false,
		display: 'auto',
		id: String(input.time_slot.id),
		resources: [],
		start: new Date(input.time_slot.start),
		end: new Date(input.time_slot.end),
		backgroundColor: input.reservation_type.color,
		...(title !== undefined ? { title } : {})
	};
}

export interface EditRegionInput {
	id: number;
	name?: string;
}

export interface EditTeamInput {
	id: number;
	name?: string;
	tags?: string[];
}

export interface Target {
	id: number;
	maybe_reservation_type?: number;
}

export interface TargetExtension {
	target: Target;
	groups: TeamGroup[];
}

export interface DuplicateEntry {
	team_groups: TeamGroup[];
	used_by: TargetExtension[];
	teams_with_group_set:
	| {
		Interregional: number;
	}
	| {
		Regional: [number, number][];
	};
}

export function totalNumberOfTeamsWithGroupset(duplicate: DuplicateEntry): number {
	if ('Interregional' in duplicate.teams_with_group_set) {
		return duplicate.teams_with_group_set.Interregional;
	}

	let result = 0;

	for (const [_regionId, count] of duplicate.teams_with_group_set.Regional) {
		result += count;
	}

	return result;
}

export interface PreScheduleReport {
	target_duplicates: DuplicateEntry[];
	target_has_duplicates: number[];
	target_required_matches: [TargetExtension, number][];
	total_matches_required: number;
	total_matches_supplied: number;
}

export interface PreScheduleReportInput {
	matches_to_play: number;
	interregional: boolean;
}

export interface ReservationType {
	id: number;
	name: string;
	color: string;
	default_sizing: number;
	description?: string;
}

export interface CreateReservationTypeInput {
	name: string;
	color: string;
	description?: string;
}

export const MAX_GAMES_PER_FIELD_TYPE = 8;
export const MIN_GAMES_PER_FIELD_TYPE = 1;

export interface FieldSupportedConcurrencyInput {
	reservation_type_ids: number[];
	field_id: number;
}

export interface FieldConcurrency {
	reservation_type_id: number;
	field_id: number;
	concurrency: number;
	is_custom: boolean;
}

export interface UpdateReservationTypeConcurrencyForFieldInput {
	reservation_type_id: number;
	field_id: number;
	new_concurrency: number;
}

export interface UpdateTargetReservationTypeInput {
	target_id: number;
	new_reservation_type_id: number | undefined;
}