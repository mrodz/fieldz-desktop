<script lang="ts">
	import {
		regionalUnionSumTotal,
		type PreScheduleReport,
		type TargetExtension,
		regionalUnionFormatPretty,
		type Region,
		type TeamGroup,
		type RegionalUnionU64
	} from '$lib';
	import { ProgressRadial, Table } from '@skeletonlabs/skeleton';

	export let report: PreScheduleReport;
	export let regionGetter: (regionId: number) => Promise<Region>;

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

	let numberOfTeams = (target_ext: TargetExtension) =>
		regionalUnionSumTotal(
			report?.target_duplicates.find((d) =>
				d.used_by.map((u) => u.target.id).includes(target_ext.target.id)
			)!.teams_with_group_set
		) ?? 0;

	$: radialOk =
		report.total_matches_required <= report.total_matches_supplied &&
		report.total_matches_supplied !== 0;

	function formatGroups(groups: TeamGroup[]): string {
		return groups.length > 0
			? groups.map((g) => `<span class="chip variant-filled">${g.name}</span>`).join(' ')
			: '<strong>Will Not Schedule</strong>';
	}

	const head = ['ID', 'Groups', '# of Teams', 'Matches Required'];

	async function mapper([target, union]: [TargetExtension, RegionalUnionU64]): Promise<
		[string, string, string, string]
	> {
		return [
			`${target.target.id}${target.groups.length === 0 ? ' ⚠️' : ''}`,
			formatGroups(target.groups),
			String(numberOfTeams(target)) +
				(regionalUnionSumTotal(union) === 0 ? ' (<i>not enough teams</i>)' : ''),
			await regionalUnionFormatPretty(union, regionGetter)
		];
	}
</script>

<div class="grid grid-cols-[auto_1fr] gap-16">
	<div>
		<h3 class="h4">Time Slots Supplied / Required</h3>
		<ProgressRadial
			class="mx-auto my-4"
			strokeLinecap="round"
			meter={radialOk ? 'stroke-success-500' : 'stroke-error-500'}
			track={radialOk ? 'stroke-success-500/30' : 'stroke-error-500/30'}
			value={percentFillage(report.total_matches_required, report.total_matches_supplied)}
		>
			{report.total_matches_supplied}/{report.total_matches_required}
		</ProgressRadial>
	</div>

	<div>
		<h3 class="h4">Per target</h3>

		{#await Promise.all(report?.target_required_matches.map(mapper) ?? [])}
			<ProgressRadial />
		{:then body}
			<Table class="my-4" source={{ head, body }} />
		{/await}
	</div>
</div>
