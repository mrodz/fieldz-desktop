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

export async function eventFromGame(
	input: ScheduleGame,
	teamGetter: (id: number) => Promise<TeamExtension>
): Promise<CalendarEvent> {
	let title = 'Empty';

	if (Number.isInteger(input?.team_one) && Number.isInteger(input?.team_two)) {
		const teamOne = await teamGetter(input.team_one!);
		const teamTwo = await teamGetter(input.team_two!);

		title = `${teamOne.team.name} vs ${teamTwo.team.name}`;
	}

	return {
		allDay: false,
		display: 'auto',
		id: `schedule-${input.schedule_id}-game-${input.id}`,
		resources: [],
		start: new Date(input.start),
		end: new Date(input.end),
		backgroundColor: title === 'Empty' ? '#808080' : 'hsl(102,21%,49%)',
		title
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
	maybe_reservation_type: number | undefined;
}

export interface TargetExtension {
	target: Target;
	groups: TeamGroup[];
}

export type RegionalUnionU64 =
	| {
			Interregional: number;
	  }
	| {
			Regional: [number, number][];
	  };

export interface DuplicateEntry {
	team_groups: TeamGroup[];
	used_by: TargetExtension[];
	teams_with_group_set: RegionalUnionU64;
}

export function regionalUnionSumTotal(union: RegionalUnionU64): number {
	if ('Interregional' in union) {
		return union.Interregional;
	}

	let result = 0;

	for (const [_regionId, count] of union.Regional) {
		result += count;
	}

	return result;
}

export async function regionalUnionFormatPretty(
	regionGetter: (regionId: number) => Promise<Region>,
	requiredUnion: RegionalUnionU64,
	suppliedUnion: RegionalUnionU64
): Promise<string> {
	if ('Interregional' in requiredUnion && 'Interregional' in suppliedUnion) {
		return String(requiredUnion.Interregional);
	}

	if (!('Regional' in requiredUnion && 'Regional' in suppliedUnion)) {
		throw new Error(`${requiredUnion} and ${suppliedUnion} were not the same type.`);
	}

	if (requiredUnion.Regional.length === 0) {
		return '0 (No region dependents)';
	}

	// we have to use inline-style because otherwise the Tailwind minifier will delete the classes.
	let result = '<div style="display: flex; flex-direction: column;">';

	let suppliedMap = new Map(suppliedUnion.Regional);

	for (const [regionId, count] of requiredUnion.Regional) {
		const region = await regionGetter(regionId);
		const supplied = suppliedMap.get(regionId) ?? 0;
		result += `<div>${region.title} &mdash; <span style="color: ${supplied < count ? 'red' : 'unset'}">${supplied}/${count}</span></div>`;
	}

	return result + '</div>';
}

export interface SupplyRequireEntry {
	target: TargetExtension;
	required: RegionalUnionU64;
	supplied: RegionalUnionU64;
}

export function isSupplyRequireEntryAccountedFor(supplyRequireEntry: SupplyRequireEntry): boolean {
	if (
		'Interregional' in supplyRequireEntry.supplied &&
		'Interregional' in supplyRequireEntry.required
	) {
		return supplyRequireEntry.supplied >= supplyRequireEntry.required;
	}

	if (!('Regional' in supplyRequireEntry.supplied && 'Regional' in supplyRequireEntry.required)) {
		throw new Error(
			`${supplyRequireEntry.supplied} and ${supplyRequireEntry.required} were not the same type.`
		);
	}

	let suppliedMap = new Map(supplyRequireEntry.supplied.Regional);

	for (const [regionId, count] of supplyRequireEntry.required.Regional) {
		const supplied = suppliedMap.get(regionId) ?? 0;
		if (supplied < count) {
			return false;
		}
	}

	return true;
}

export interface PreScheduleReport {
	target_duplicates: DuplicateEntry[];
	target_has_duplicates: number[];
	target_match_count: SupplyRequireEntry[];
	// target_required_matches: [TargetExtension, RegionalUnionU64][];
	total_matches_required: number;
	total_matches_supplied: number;
	interregional: boolean;
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

export const HAS_DB_RESET_BUTTON: boolean = false;
export const TIME_SLOT_CREATION_MODAL_ENABLE: boolean = false;
export const SCHEDULE_CREATION_DELAY: number = 30_000;
export const SHOW_SCHEDULER_JSON_PAYLOADS: boolean = false;
export const SHOW_SCHEDULER_URL_WHILE_WAITING: boolean = false;

export interface FieldExtension {
	field_id: number;
	time_slots: TimeSlotExtension[];
}

export interface PlayableTeamCollection {
	tags: TeamGroup[];
	teams: TeamExtension[];
}

export interface ScheduledInput {
	team_groups: PlayableTeamCollection[];
	fields: FieldExtension[];
}

export interface Schedule {
	id: number;
	name: string;
	created: string;
	last_edit: string;
}

export interface EditScheduleInput {
	id: number;
	name: string;
}

export type HealthCheck = 'Serving' | 'NotServing' | 'Unknown';

export interface DateRange {
	start: Date;
	end: Date;
}

export interface Delta {
	years: number;
	months: number;
	days: number;
	seconds: number;
	inWeeks: boolean;
}

export interface ScheduleGame {
	id: number;
	schedule_id: number;
	start: string;
	end: string;
	team_one?: number;
	team_two?: number;
}

export interface GoogleOAuthAccessTokenExchange {
	access_token: string,
	expires_in: number,
	id_token: string | undefined,
	refresh_token: string,
	scope: string,
	token_type: string,
}