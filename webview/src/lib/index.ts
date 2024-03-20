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

export interface CreateTimeSlotInput {
	field_id: number;
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
	'#86a86c'
];

function colorForTimeSlot(input: TimeSlot): string {
	return CALENDAR_TIME_SLOT_COLORS[input.field_id % CALENDAR_TIME_SLOT_COLORS.length];
}

export function eventFromTimeSlot(input: TimeSlot, title?: string): CalendarEvent {
	return {
		allDay: false,
		display: 'auto',
		id: String(input.id),
		resources: [],
		start: new Date(input.start),
		end: new Date(input.end),
		backgroundColor: colorForTimeSlot(input),
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
}

export interface TargetExtension {
	target: Target;
	groups: TeamGroup[];
}

export interface DuplicateEntry {
	team_groups: TeamGroup[];
	used_by: TargetExtension[];
	teams_with_group_set: number;
}

export interface PreScheduleReport {
	target_duplicates: DuplicateEntry[];
	target_has_duplicates: number[];
	target_required_matches: [TargetExtension, number][];
	total_matches_required: number;
	total_matches_supplied: number;
}
