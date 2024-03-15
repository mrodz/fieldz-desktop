<script lang="ts">
	import type { TeamGroup } from '$lib';
	import {
		Autocomplete,
		popup,
		type AutocompleteOption,
		type PopupSettings
	} from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';

	export let groups: TeamGroup[];

	const dispatch = createEventDispatcher();

	let inputChipList: TeamGroup[] = [];

	let inputChip = '';

	$: flavorOptions = (groups ?? []).map(
		(group) =>
			({
				label: group.name,
				value: group.name,
				keywords: '',
				meta: {
					group
				}
			}) satisfies AutocompleteOption<string, { group: TeamGroup }>
	);

	function onFlavorSelection(
		event: CustomEvent<AutocompleteOption<string, { group: TeamGroup }>>
	): void {
		// inputChipList.push(event.detail.label);
		inputChip = '';
		inputChipList.push(event.detail.meta!.group);
		inputChipList = inputChipList;
	}

	let popupSettings: PopupSettings = {
		event: 'focus-click',
		target: 'popupAutocomplete',
		placement: 'bottom'
	};

	function onSubmit() {
		dispatch('submit', {
			groups: inputChipList
		});
	}
</script>

<div class="border-l-4 border-green-400 pl-4">
	A schedule will be created for all teams with these groups:
	<ol class="list my-4">
		{#if inputChipList.length === 0}
			<span class="block text-center"><i>Nothing in this target yet</i> &#129431;</span>
		{/if}
		{#each inputChipList as chip, i}
			<li>
				<span class="badge-icon variant-filled">{i + 1}</span>
				<span class="flex-auto">{chip.name}</span>
			</li>
		{/each}
	</ol>

	<div class="grid grid-cols-[1fr_auto] gap-2">
		<input
			class="input autocomplete"
			type="search"
			name="autocomplete-search"
			bind:value={inputChip}
			placeholder="Group Name..."
			use:popup={popupSettings}
		/>
		<button class="btn variant-filled" on:click={() => onSubmit()}> Submit Target </button>
	</div>

	<div data-popup="popupAutocomplete" class="card p-4">
		<Autocomplete
			bind:input={inputChip}
			options={flavorOptions}
			denylist={inputChipList.map((group) => group.name)}
			emptyState={'<span>⚠️ You can only use a group once</span>'}
			on:selection={onFlavorSelection}
		/>
	</div>
</div>
