<script lang="ts">
	import { goto } from '$app/navigation';
	import type { TeamExtension } from '$lib';
	import { getModalStore, getToastStore, type AutocompleteOption } from '@skeletonlabs/skeleton';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	export let parent: any;

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	const regionIdAny = $modalStore[0].meta.regionId;

	if (regionIdAny === undefined || !Number.isInteger(Number(regionIdAny))) {
		throw new Error('Missing regionId');
	}

	const regionId = Number(regionIdAny);

	let teams: Promise<TeamExtension[]> | undefined;
	let teamOptions: Promise<AutocompleteOption<string, TeamExtension>[]> | undefined;

	onMount(async () => {
		teams = invoke('get_teams_and_tags', { regionId });
		teamOptions = teams.then((teams) => {
			let options: AutocompleteOption<string, TeamExtension>[] = [];

			for (const team_ext of teams) {
				// format: 'Item1, Item2, Item3'
				let keywords = '';

				let [first, ...rest] = team_ext.tags;

				if (first !== undefined) {
					keywords += first.name;
				}

				for (const tag of rest) {
					keywords += ', ';
					keywords += tag.name;
				}

				options.push({
					label: team_ext.team.name,
					value: team_ext.team.name,
					keywords,
					meta: team_ext
				})
			}

			return options;
		});
	});
</script>

<div class="card w-modal p-5">

</div>
