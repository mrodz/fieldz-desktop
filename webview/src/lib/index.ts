// db\entity\src\entities\region.rs
export interface Region {
	id: number;
	title: string;
}

export interface CreateRegionInput {
	title: string;
}

export interface Field {
	id: number,
	name: string,
	region_owner: number,
}

export interface CreateFieldInput {
	name: string,
	region_id: number
}