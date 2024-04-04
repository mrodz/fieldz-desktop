<script lang="ts">
	import { totalNumberOfTeamsWithGroupset, type PreScheduleReport } from '$lib';

	export let report: PreScheduleReport;

	function reportHasErrors(report: PreScheduleReport): boolean {
		return (
			report.target_has_duplicates.length !== 0 ||
			report.target_duplicates.find((d) => totalNumberOfTeamsWithGroupset(d) === 0) !== undefined ||
			report.target_required_matches.find(([_, occ]) => occ === 0) !== undefined
		);
	}
</script>

{#if reportHasErrors(report)}
	<div class="card bg-error-400 m-4 grid gap-4 p-4 text-center">
		{#if report.target_has_duplicates.length !== 0}
			<div>
				<strong>Cannot use targets because of duplicates</strong>
				<ul class="list">
					{#each report.target_duplicates.filter((d) => d.used_by.length > 1) as dup}
						<li>
							<span>Duplicates on {dup.team_groups.length === 0 ? 'empty' : ''} targets</span>

							{#each dup.used_by as badTarget}
								<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
									>{badTarget.target.id}</a
								>
							{/each}

							{#if dup.team_groups.length !== 0}
								<span>which use the following labels:</span>

								{#each dup.team_groups as group}
									<span class="variant-filled chip">{group.name}</span>
								{/each}
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if report.target_duplicates.find((d) => totalNumberOfTeamsWithGroupset(d) === 0) !== undefined}
			<div>
				<strong>Cannot use targets because no team has the following sets of labels</strong>
				<ul class="list">
					{#each report.target_duplicates.filter((d) => totalNumberOfTeamsWithGroupset(d) === 0) as empty}
						<li>
							<span>Target(s)</span>

							{#each empty.used_by as badTarget}
								<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
									>{badTarget.target.id}</a
								>
							{/each}

							{#if empty.team_groups.length === 0}
								<span>reference(s) impossible team which uses no labels</span>
							{:else}
								<span>reference(s) impossible team which uses labels</span>

								{#each empty.team_groups as group}
									<span class="variant-filled chip">{group.name}</span>
								{/each}
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if report.target_required_matches.find(([_, occ]) => occ === 0) !== undefined}
			<div>
				<strong>Cannot use targets because no games will be outputted</strong>
				<ul class="list">
					{#each report.target_required_matches.filter(([_, occ]) => occ === 0) as [badTarget]}
						<li>
							<span>Target</span>

							<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
								>{badTarget.target.id}</a
							>

							{#if badTarget.groups.length === 0}
								<span>is empty and will not create any games</span>
							{:else}
								<span>which use labels</span>

								{#each badTarget.groups as group}
									<span class="variant-filled chip">{group.name}</span>
								{/each}

								<span>will not create any games</span>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
{/if}
