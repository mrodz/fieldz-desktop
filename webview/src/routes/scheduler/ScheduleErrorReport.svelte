<script lang="ts">
	import {
		regionalUnionSumTotal,
		type PreScheduleReport,
		isSupplyRequireEntryAccountedFor
	} from '$lib';

	export let report: PreScheduleReport;
	export let hasErrors = false;

	function reportHasErrors(report: PreScheduleReport): boolean {
		return (
			report.target_has_duplicates.length !== 0 ||
			report.target_duplicates.find((d) => regionalUnionSumTotal(d.teams_with_group_set) === 0) !==
				undefined ||
			report.target_match_count.find((req) => regionalUnionSumTotal(req.required) === 0) !==
				undefined ||
			!report.target_match_count.every(isSupplyRequireEntryAccountedFor)
		);
	}

	$: hasErrors = reportHasErrors(report);
</script>

{#if hasErrors}
	<div class="card m-4 grid gap-4 bg-error-400 p-4 text-center">
		{#if report.target_has_duplicates.length !== 0}
			<div>
				<strong>Cannot use targets because of duplicates</strong>
				<ul class="list">
					{#each report.target_duplicates.filter((d) => d.used_by.length > 1) as dup}
						<li>
							<span>Duplicates on {dup.team_groups.length === 0 ? 'empty' : ''} targets</span>

							{#each dup.used_by as badTarget}
								<a
									class="variant-filled-error chip"
									href="#target-{badTarget.target.id}"
									on:click|stopPropagation>{badTarget.target.id}</a
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
		{#if report.target_duplicates.find((d) => regionalUnionSumTotal(d.teams_with_group_set) === 0) !== undefined}
			<div>
				<strong>Cannot use targets because no team has the following sets of labels</strong>
				<ul class="list">
					{#each report.target_duplicates.filter((d) => regionalUnionSumTotal(d.teams_with_group_set) === 0) as empty}
						<li>
							<span>Target(s)</span>

							{#each empty.used_by as badTarget}
								<a
									class="variant-filled-error chip"
									href="#target-{badTarget.target.id}"
									on:click|stopPropagation>{badTarget.target.id}</a
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
		{#if report.target_match_count.find((req) => regionalUnionSumTotal(req.required) === 0) !== undefined}
			<div>
				<strong>Cannot use targets because no games will be outputted</strong>
				<ul class="list">
					{#each report.target_match_count.filter((req) => regionalUnionSumTotal(req.required) === 0) as supplyReqEntry}
						<li>
							<span>Target</span>

							<a
								class="variant-filled-error chip"
								href="#target-{supplyReqEntry.target.target.id}"
								on:click|stopPropagation>{supplyReqEntry.target.target.id}</a
							>

							{#if supplyReqEntry.target.groups.length === 0}
								<span>is empty and will not create any games</span>
							{:else}
								<span>which use labels</span>

								{#each supplyReqEntry.target.groups as group}
									<span class="variant-filled chip">{group.name}</span>
								{/each}

								<span>will not create any games</span>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if !report.target_match_count.every(isSupplyRequireEntryAccountedFor)}
			<div>
				<strong>
					Cannot proceed with scheduling because not every region supplies enough time slots
				</strong>

				<ul class="list">
					{#each report.target_match_count.filter((req) => !isSupplyRequireEntryAccountedFor(req)) as supplyReqEntry}
						<li>
							<span>Target</span>

							<a
								class="variant-filled-error chip"
								href="#target-{supplyReqEntry.target.target.id}"
								on:click|stopPropagation>{supplyReqEntry.target.target.id}</a
							>

							<span>which use labels</span>

							{#each supplyReqEntry.target.groups as group}
								<span class="variant-filled chip">{group.name}</span>
							{/each}

							<span>requires games in regions that do not provide enough time slots</span>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
{/if}
