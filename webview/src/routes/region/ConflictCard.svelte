<script lang="ts">
	import type { CoachingConflict, TeamExtension } from '$lib';
	import { faArrowUp, faSpinner, faTrash, faX } from '@fortawesome/free-solid-svg-icons';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';

	export let teamById: (id: number) => TeamExtension;
	export let conflict: CoachingConflict;
	const mappingNameOnLoad = conflict.coach_name;

	let reportTimer: NodeJS.Timeout | undefined;
	let pendingPost: boolean = false;
	let deleting = false;

	let modalStore = getModalStore();

	let dispatch = createEventDispatcher<{
		delete: CoachingConflict;
		debouncedUpdate: {
			conflict: CoachingConflict;
			options: {
				nameOnLoad: string | undefined;
			};
		};
		addTeam: {
			addTeamToConflict: typeof addTeamToConflict;
		};
		removeTeam: {
			teamId: number;
		};
	}>();

	function onDelete() {
		deleting = true;
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm',
			body: 'Deleting a coach mapping is PERMANENT! Are you sure you wish to proceed?',
			response(r) {
				if (r) {
					dispatch('delete', conflict);
				} else {
					deleting = false;
				}
			}
		});
	}

	function requestUpdate() {
		clearTimeout(reportTimer);
		pendingPost = true;
		reportTimer = setTimeout(async () => {
			dispatch('debouncedUpdate', {
				conflict,
				options: {
					nameOnLoad: mappingNameOnLoad
				}
			});
			pendingPost = false;
		}, 1_000);
	}

	function addTeamToConflict(team_ext: TeamExtension) {
		conflict.teams.push(team_ext.team);
		conflict = conflict;
	}

	function onAddTeam() {
		dispatch('addTeam', { addTeamToConflict });
	}

	function onRemoveTeam(teamId: number) {
		dispatch('removeTeam', { teamId });
		conflict.teams = conflict.teams.filter((team) => team.id !== teamId);
	}
</script>

<div class="card m-4 p-4">
	<div class="grid grid-cols-[1fr_auto] gap-4">
		<input
			class="w-full border-none bg-transparent"
			bind:value={conflict.coach_name}
			on:keydown={() => requestUpdate()}
			placeholder="Give this coach a name (Optional)"
		/>
		{#if pendingPost}
			<div class="flex flex-col items-center">
				<Fa class="m-auto inline animate-spin" size="lg" icon={faSpinner} />
				Saving
			</div>
		{:else}
			<button class="btn-icon-md variant-filled btn-icon m-auto" on:click={() => onDelete()}>
				<Fa class="inline" size="sm" icon={faTrash} />
				<span class="sr-only">Delete coach mapping</span>
			</button>
		{/if}
		<dl class="list-dl">
			{#each conflict.teams as teamId}
				{@const team_ext = teamById(teamId.id)}
				<div>
					{#if pendingPost}
						<div class="flex flex-col items-center" aria-disabled="true">
							<Fa class="inline" size="lg" icon={faArrowUp} />
							<span class="sr-only">Please wait</span>
						</div>
					{:else}
						<button class="btn-icon-md btn-icon m-auto" on:click={() => onRemoveTeam(teamId.id)}>
							<Fa class="inline" size="xs" icon={faX} />
							<span class="sr-only">Remove team</span>
						</button>
					{/if}
					<span class="flex-auto">
						<dt>{team_ext?.team?.name}</dt>
						<dd>
							{#if (team_ext?.tags ?? []).length !== 0}
								<div>
									{#each team_ext?.tags ?? [] as tag}
										<span class="variant-filled-success chip">{tag.name}</span>
									{/each}
								</div>
							{:else}
								<i>No groups yet!</i>
							{/if}
						</dd>
					</span>
				</div>
			{:else}
				<div class="text-center block mx-auto">No teams added!</div>
			{/each}
		</dl>
		<div class="my-auto flex flex-col">
			<button class="variant-filled btn-icon mx-auto block" on:click={onAddTeam}>+</button>
			<span class="mx-auto mt-2 block">Add team</span>
		</div>
	</div>
</div>
