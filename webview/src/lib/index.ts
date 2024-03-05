export interface Region {
	id: number,
	title: string,
}

export interface CreateRegionInput {
	title: string,
}

export interface Field {
	id: number,
	name: string,
	region_owner: number,
}

export interface CreateFieldInput {
	name: string,
	region_id: number,
}

export interface TeamGroup {
	id: number,
	name: string,
	usages: number,
}

export interface TeamExtension {
	team: Team,
	tags: TeamGroup[],
}

export interface Team {
	id: number,
	name: string,
	region_owner: number,
}

export interface CreateTeamInput {
	name: string,
	region_id: number,
	tags: string[],
}

export interface TeamGroup {
	id: number,
	name: string,
}

export interface TimeSlot {
	id: number,
	field_id: number,
	start: string,
	end: string,
}

export interface CreateTimeSlotInput {
	field_id: number,
	start: number,
	end: number,
}

/**
 * Reference: {@link https://github.com/vkurko/calendar?tab=readme-ov-file#event-object}
 */
export interface CalendarEvent {
	id: string,
	resources: unknown[],
	allDay: boolean,
	start: Date,
	end: Date,
	title?: string | { html: string } | { domNodes: Node[] },
	editable?: boolean,
	startEditable?: boolean,
	durationEditable?: boolean,
	display: 'auto' | 'background',
	backgroundColor?: string,
	eventTextColor?: string,
	extendedProps?: any
}

export interface MoveTimeSlotInput {
	field_id: number,
	id: number,
	new_start: number,
	new_end: number,
}