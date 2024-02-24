<script lang="ts">
	import type { TeamGroup } from '$lib';
	import { InputChip } from '@skeletonlabs/skeleton';
	import { slide } from 'svelte/transition';
	import { invoke, dialog } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	let groups: TeamGroup[] = [];
	$: groupsFrontend = groups.map((group) => group.name);

	onMount(async () => {
		try {
			groups = await invoke<TeamGroup[]>('get_groups');
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Error getting team groups',
				type: 'error'
			});
		}
	});

	/**
	 * Minimal validation; real validation happens in Rust
	 *
	 * @param value A string from the `InputChip`
	 */
	function noDuplicates(value: string): boolean {
		for (let tag of groups) {
			if (tag.name.toLowerCase() === value.toLowerCase()) {
				return false;
			}
		}

		return true;
	}

	async function add(tag: string) {
		tag = tag.toLowerCase();

		try {
			const newGroup = await invoke<TeamGroup>('create_group', { tag });
			groups.push(newGroup);
			groups = groups;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not add group',
				type: 'error'
			});
		}
	}

	async function remove(index: number) {
		const groupToDelete = groups[index];

		try {
			await invoke('delete_group', { id: groupToDelete.id });
			groups.splice(index, 1);
			groups = groups;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not delete group',
				type: 'error'
			});
		}
	}
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h2 class="h2">Groups</h2>

	<div class="[&>*]:my-4">
		<p>
			Here is where you create labels you'll use to group teams
			<strong>across regions</strong>. Teams that you create in any region can have many grouping labels.
		</p>
		<p>Some use cases might be:</p>
		<ul class="list">
			<li>&bull; Age Groups (u8, u10, u12, etc)</li>
			<li>&bull; Extras or All Stars</li>
			<li>&bull; Tournament Brackets</li>
		</ul>
	</div>

	<InputChip
		bind:value={groupsFrontend}
		on:add={(customEvent) => {
			add(customEvent.detail.chipValue);
		}}
		on:remove={(customEvent) => {
			remove(customEvent.detail.chipIndex);
		}}
		validation={noDuplicates}
		class="mt-4"
		name="groups"
		placeholder="Start typing a label, then hit enter"
		{...$$restProps}
	/>
</main>
