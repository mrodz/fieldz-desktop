<script lang="ts">
	import type { Team, TeamExtension } from '$lib';
	import {
		Autocomplete,
		getModalStore,
		ProgressRadial,
		type AutocompleteOption
	} from '@skeletonlabs/skeleton';
	import { invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';

	export let parent: any;

	const modalStore = getModalStore();

	let { regionId, onTeamSelected, excludeTeams } = $modalStore[0].meta as {
		regionId: number | string;
		onTeamSelected: (team: TeamExtension) => void;
		excludeTeams?: (number | Team)[];
	};

	excludeTeams = excludeTeams ?? [];

	if (regionId === undefined || !Number.isInteger(Number(regionId))) {
		console.error('Missing regionId, or non-integer');
		throw new TypeError('Missing regionId, or non-integer');
	}

	if (typeof onTeamSelected !== 'function') {
		console.error('onTeamSelected is not a function');
		throw new TypeError('onTeamSelected is not a function');
	}

	regionId = Number(regionId);

	let teams: Promise<TeamExtension[]> | undefined;
	let teamOptions: Promise<AutocompleteOption<string, TeamExtension>[]> | undefined;

	let teamPicked: TeamExtension | undefined;

	onMount(async () => {
		teams = invoke('get_teams_and_tags', { regionId });
		teamOptions = teams.then((teams) => {
			let options: AutocompleteOption<string, TeamExtension>[] = [];

			for (const team_ext of teams) {
				const thisId = team_ext.team.id;
				const isExcluded = excludeTeams!.some((idOrTeam) => {
					if (typeof idOrTeam === 'number' && idOrTeam === thisId) return true;
					return typeof idOrTeam === 'object' && idOrTeam.id === thisId;
				});

				if (isExcluded) continue;

				const option: AutocompleteOption<string, TeamExtension> = {
					label: team_ext.team.name,
					value: team_ext.team.name,
					meta: team_ext
				};

				let [first, ...rest] = team_ext.tags;

				if (first !== undefined) {
					// format: 'Item1, Item2, Item3'
					let keywords = first.name;

					for (const tag of rest) {
						keywords += ', ';
						keywords += tag.name;
					}

					option.keywords = keywords;
				}

				options.push(option);
			}

			return options;
		});
	});

	function close() {
		parent.onClose();
	}

	let nameInput: string | undefined;

	function onComponentTeamSelection(event: CustomEvent<AutocompleteOption<string, TeamExtension>>) {
		nameInput = event.detail.label;
		teamPicked = event.detail.meta;
	}

	function confirm() {
		if (teamPicked !== undefined) onTeamSelected?.(teamPicked);
		close();
	}
</script>

<div class="card w-modal p-5">
	<h2 class="h2">
		Pick Team
		{#if teamPicked !== undefined}
			(selected)
		{/if}
	</h2>

	{#if teams !== undefined && teamOptions !== undefined}
		{#await Promise.all([teams, teamOptions])}
			<div class="align-center flex">
				<ProgressRadial />
			</div>
		{:then [teams, teamOptions]}
			<input
				class="input my-4"
				type="search"
				name="teamName"
				bind:value={nameInput}
				placeholder="Search team name..."
			/>

			<div class="card max-h-48 w-full overflow-y-auto p-4" tabindex="-1">
				<Autocomplete
					bind:input={nameInput}
					options={teamOptions}
					on:selection={onComponentTeamSelection}
				/>
			</div>
		{/await}
	{/if}

	<hr class="hr my-5" />

	<div class="grid grid-cols-[1fr_auto] items-center">
		<div>
			{#if teamPicked !== undefined}
				<div class="card px-4 py-2">
					{teamPicked.team.name}
					<div>
						{#each teamPicked.tags as tag, i}
							<span
								in:fade={{ delay: i * 100 + 100 }}
								out:fade={{ delay: i * 100 + 100 }}
								class="variant-filled-success chip">{tag.name}</span
							>
						{/each}
					</div>
				</div>
			{/if}
		</div>
		<div class="flex flex-row-reverse [&>button]:h-min">
			<button class="variant-filled btn" disabled={teamPicked === undefined} on:click={confirm}
				>Confirm</button
			>
			<button class="variant-outline btn mx-1" on:click={close}>Cancel</button>
		</div>
	</div>
</div>
