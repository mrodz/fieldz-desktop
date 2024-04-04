<script lang="ts">
	import {
		totalNumberOfTeamsWithGroupset,
		type PreScheduleReport,
		type TargetExtension
	} from '$lib';
	import { ProgressRadial, Table } from '@skeletonlabs/skeleton';

	export let report: PreScheduleReport;

	function percentFillage(totalMatchesRequired: number, totalMatchesSupplied: number): number {
		if (totalMatchesRequired === 0) {
			totalMatchesSupplied = 1;
		} else {
			totalMatchesSupplied /= totalMatchesRequired;
		}

		return Math.round(totalMatchesSupplied * 100);
	}

	let numberOfTeams = (target_ext: TargetExtension) =>
		totalNumberOfTeamsWithGroupset(
			report?.target_duplicates.find((d) =>
				d.used_by.map((u) => u.target.id).includes(target_ext.target.id)
			)!
		) ?? 0;

	$: radialOk =
		report.total_matches_required <= report.total_matches_supplied &&
		report.total_matches_supplied !== 0;
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

		<Table
			class="my-4"
			source={{
				head: ['ID', 'Groups', '# of Teams', 'Matches Required'],
				body:
					report?.target_required_matches.map(([target, matches]) => [
						`${target.target.id}${target.groups.length === 0 ? ' ⚠️' : ''}`,
						target.groups.length > 0
							? target.groups
									.map((g) => `<span class="chip variant-filled">${g.name}</span>`)
									.join(' ')
							: '<strong>Will Not Schedule</strong>',
						String(numberOfTeams(target)) + (matches === 0 ? ' (<i>not enough teams</i>)' : ''),
						String(matches)
					]) ?? []
			}}
		/>
	</div>
</div>
