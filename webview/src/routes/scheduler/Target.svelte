<script lang="ts">
	import type { TargetExtension, TeamGroup } from '$lib';
	import { faCircleXmark, faTrash } from '@fortawesome/free-solid-svg-icons';
	import {
		Autocomplete,
		popup,
		type AutocompleteOption,
		type PopupSettings
	} from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';
	import { blur } from 'svelte/transition';

	export let target: TargetExtension;
	export let groups: TeamGroup[];
	export let popupId: any;
	export let ok: boolean;

	const dispatch = createEventDispatcher<{
		groupAdd: TeamGroup,
		groupDelete: TeamGroup,
		delete: TargetExtension,
	}>();

	let inputChipList: TeamGroup[] = [...target.groups];

	let inputChip = '';

	function onGroupSelection(
		event: CustomEvent<AutocompleteOption<string, { group: TeamGroup }>>
	): void {
		inputChip = '';
		inputChipList.push(event.detail.meta!.group);
		inputChipList = inputChipList;

		dispatch('groupAdd', {
			...event.detail.meta!.group
		});
	}

	function onGroupDeletion(group: TeamGroup, index: number) {
		inputChipList.splice(index, 1);
		inputChipList = inputChipList;

		dispatch('groupDelete', {
			...group
		});
	}

	let popupSettings: PopupSettings = {
		event: 'focus-click',
		target: `popupAutocomplete-${popupId}`,
		placement: 'bottom'
	};

	function onDelete() {
		dispatch('delete', {
			...target
		});
	}
</script>

<div
	out:blur={{ opacity: 0.5, duration: 100 }}
	class="grid grid-cols-[auto_1fr] gap-4"
	{...$$restProps}
>
	<div class="pt-2">
		<strong class="ml-auto inline-block text-center" in:blur={{ opacity: 0.5 }}
			>ID. {target.target.id}</strong
		>
		<button class="btn-icon ml-auto mt-4 flex flex-col" on:click={onDelete}>
			<Fa class="inline" size="lg" icon={faCircleXmark} />
			Remove
		</button>
	</div>
	<div class="flex flex-col border-l-4 {ok ? 'border-green-400' : 'border-red-400'} pl-4 pt-4">
		A unique schedule will be created for all teams with these groups:
		{#if inputChipList.length === 0}
			<!-- &#129431; -->
			<div class="my-auto block p-6 text-center">⚠️ <i>Empty target, will skip</i></div>
		{:else}
			<ol class="list my-4 grow">
				{#each inputChipList as group, i}
					<li>
						<button class="btn-icon" on:click={() => onGroupDeletion(group, i)}>
							<Fa class="inline" size="sm" icon={faTrash} />
						</button>
						<!-- <span class="badge-icon variant-filled">{i + 1}</span> -->
						<span class="flex-auto">{group.name}</span>
					</li>
				{/each}
			</ol>
		{/if}

		<input
			class="autocomplete input"
			type="search"
			name="autocomplete-search"
			autocomplete="off"
			bind:value={inputChip}
			placeholder="Select Group Name..."
			use:popup={popupSettings}
		/>

		<div data-popup="popupAutocomplete-{popupId}" class="card p-4">
			<Autocomplete
				bind:input={inputChip}
				options={groups.map((group) => ({
					label: group.name,
					value: group.name,
					keywords: '',
					meta: {
						group
					}
				}))}
				denylist={inputChipList.map((group) => group.name)}
				emptyState={'<span class="z-50">⚠️ You can only use a group once</span>'}
				on:selection={onGroupSelection}
			/>
		</div>
	</div>
</div>
