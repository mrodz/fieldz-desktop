<script lang="ts">
	import {
		regionalUnionSumTotal,
		type PreScheduleReport,
		regionalUnionFormatPretty,
		type Region,
		type TeamGroup,
		type SupplyRequireEntry,
		isSupplyRequireEntryAccountedFor,
		type ReservationType
	} from '$lib';
	import { ProgressRadial, Table } from '@skeletonlabs/skeleton';

	export let report: PreScheduleReport;
	export let regionGetter: (regionId: number) => Promise<Region>;
	export let reservationTypeGetter: (reservationTypeId: number) => ReservationType | undefined;
	export let previousReport: PreScheduleReport | undefined = undefined;

	/**
	 * @param totalMatchesRequired the total number of matches required to create a schedule, as an integer
	 * @param totalMatchesSupplied the total number of matches that have been supplied, as an integer
	 * @returns the percentage as an integer in the interval of [0, 100]
	 */
	function percentFillage(totalMatchesRequired: number, totalMatchesSupplied: number): number {
		if (totalMatchesRequired === 0) {
			totalMatchesSupplied = 1;
		} else {
			totalMatchesSupplied /= totalMatchesRequired;
		}

		return Math.min(Math.round(totalMatchesSupplied * 100), 100);
	}

	$: radialOk =
		report.total_matches_required <= report.total_matches_supplied &&
		report.total_matches_supplied !== 0 &&
		report.target_match_count.every(isSupplyRequireEntryAccountedFor) &&
		report.target_match_count.length !== 0;

	function formatGroups(groups: TeamGroup[]): string {
		return groups.length > 0
			? groups.map((g) => `<span class="chip variant-filled">${g.name}</span>`).join(' ')
			: '<strong>Will Not Schedule</strong>';
	}

	const head = [
		'ID',
		'Field Type',
		'Groups',
		'# of Teams',
		`Matches ${report.interregional ? 'Required' : 'Supplied/Required (Per Region)'}`
	];

	async function mapper(
		supReqEntry: SupplyRequireEntry
	): Promise<[string, string, string, string, string]> {
		const thisTargetDup = report?.target_duplicates.find((d) =>
			d.used_by.map((u) => u.target.id).includes(supReqEntry.target.target.id)
		)!;

		const maybeReservationType = supReqEntry.target.target.maybe_reservation_type;

		const reservationType =
			maybeReservationType === undefined
				? 'All'
				: reservationTypeGetter(maybeReservationType)?.name ?? 'All';

		return [
			`${supReqEntry.target.target.id}${supReqEntry.target.groups.length === 0 ? ' ⚠️' : ''}`,
			reservationType,
			formatGroups(supReqEntry.target.groups),
			String(regionalUnionSumTotal(thisTargetDup.teams_with_group_set)) +
				(regionalUnionSumTotal(supReqEntry.required) === 0 ? ' (<i>not enough teams</i>)' : ''),
			await regionalUnionFormatPretty(regionGetter, supReqEntry.required, supReqEntry.supplied)
		];
	}

	// would otherwise be negative if not enough time slots are supplied in the previous stage.
	let totalMatchesSupplied = Math.max(
		report.total_matches_supplied - (previousReport?.total_matches_required ?? 0),
		0
	);
</script>

<div class="grid grid-cols-[auto_1fr] gap-16">
	<div>
		<h3 class="h4">Time Slots Supplied / Required</h3>
		<ProgressRadial
			class="mx-auto my-4"
			strokeLinecap="round"
			meter={radialOk ? 'stroke-success-500' : 'stroke-error-500'}
			track={radialOk ? 'stroke-success-500/30' : 'stroke-error-500/30'}
			value={percentFillage(report.total_matches_required, totalMatchesSupplied)}
		>
			{totalMatchesSupplied}/{report.total_matches_required}
		</ProgressRadial>
	</div>

	<div>
		<h3 class="h4">Per target</h3>

		{#await Promise.all(report?.target_match_count.map(mapper) ?? [])}
			<ProgressRadial />
		{:then body}
			<Table class="my-4" source={{ head, body }} />
		{/await}
	</div>
</div>
