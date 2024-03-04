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
	id: number,
	field_id: number,
	start: String,
	end: String,
}