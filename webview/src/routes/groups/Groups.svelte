<script lang="ts">
	import type { TeamGroup } from '$lib';
	import { InputChip, getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let groups: TeamGroup[] = [];
	$: groupsFrontend = groups.map((group) => `${group.name} (${group.usages})`);

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

<section class="p-4">
	<h2 class="h2">Groups</h2>

	<div class="[&>*]:my-4">
		<p>
			Here is where you create labels you'll use to group teams
			<strong>across regions</strong>. Teams that you create in any region can have many grouping
			labels.
		</p>
		<p>
			Try to make labels as generic as possible. Once you've inputted all of your data, you can
			create rules to have specific labels play each other.
		</p>
		<p>Some use cases might be:</p>
		<ul class="list">
			<li>&bull; Age Groups (u8, u10, u12, etc)</li>
			<li>&bull; Boys/Girls</li>
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
			const item = groups[customEvent.detail.chipIndex];
			let message = `Deleting a group is PERMANENT! Are you sure you wish to proceed? You will NOT be able to recover "${item.name}"`;
			if (item.usages > 0) {
				message += `, <strong><u>which ${item.usages} team${item.usages == 1 ? '' : 's'} actively depend${item.usages === 1 ? 's' : ''} on for scheduling</u></strong>. Make sure that you know what you're doing, as deleting a group can have unwanted consequences on the scheduling algorithm.`;
			}
			modalStore.trigger({
				type: 'confirm',
				title: 'Danger Zone',
				body: message,
				buttonTextConfirm: 'Delete',
				async response(r) {
					if (r) {
						remove(customEvent.detail.chipIndex);
						toastStore.trigger({
							message: `"${item.name}" is no longer a group${item.usages === 0 ? '' : `, even though it was being used across ${item.usages} teams`}`,
							background: item.usages === 0 ? 'variant-filled-success' : 'variant-filled-warning'
						});
					}
				}
			});
		}}
		validation={noDuplicates}
		class="mt-4"
		name="groups"
		placeholder="Start typing a label, then hit enter"
		{...$$restProps}
	/>
</section>
