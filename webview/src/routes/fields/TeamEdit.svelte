<script lang="ts">
	import type { EditTeamInput, TeamExtension, TeamGroup } from '$lib';
	import { faCircleInfo } from '@fortawesome/free-solid-svg-icons';
	import { ProgressRadial, getModalStore, getToastStore, popup } from '@skeletonlabs/skeleton';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import Fa from 'svelte-fa';

	export let parent: any;

	const toastStore = getToastStore();
	const modalStore = getModalStore();
	// alert(JSON.stringify($modalStore[0].meta!.team.tags));

	let teamNameInput: string = $modalStore[0].meta!.team.team.name;

	let teamNameError: string | undefined;

	function close() {
		parent.onClose();
	}

	let groups: TeamGroup[] | undefined;
	let tags: string[] = $modalStore[0].meta!.team.tags.map((tag: TeamGroup) => tag.name);

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

	async function confirm() {
		const newTags = new Set(tags);
		const oldTags = new Set<string>(
			$modalStore[0].meta.team.tags.map((tag: TeamGroup) => tag.name)
		);

		const difference = new Set(newTags.size > oldTags.size ? newTags : oldTags);

		for (const tag of newTags.size > oldTags.size ? oldTags : newTags) {
			difference.delete(tag);
		}

		if (teamNameInput === $modalStore[0].meta.team.team.name && difference.size === 0) {
			toastStore.trigger({
				message: "You didn't change anything!",
				background: 'variant-filled-warning'
			});
			return;
		}

		const input = {
			name: teamNameInput,
			id: $modalStore[0].meta.team.team.id,
			tags
		} satisfies EditTeamInput;

		try {
			const newTeam = await invoke<TeamExtension>('update_team', {
				input
			});

			$modalStore[0].meta?.onUpdate(newTeam);

			close();
		} catch (e: any) {
			/*
			 * db\entity\src\lib.rs
			 */
			if (typeof e === 'object' && 'ValidationError' in e) {
				const error = e['ValidationError'];

				if (error === 'EmptyName') {
					teamNameError = 'Team name cannot be empty';
				} else if (typeof error === 'object' && 'NameTooLong' in error) {
					const nameTooLong = error['NameTooLong'];
					teamNameError = `Team name is ${nameTooLong?.len} characters which is larger than the max, 64`;
				} else {
					// unknown validation error!
					dialog.message(JSON.stringify(e), {
						title: 'Error',
						type: 'error'
					});
				}
			} else {
				dialog.message(JSON.stringify(e), {
					title: 'Error',
					type: 'error'
				});
			}
		}
	}
</script>

<div class="card w-modal p-5">
	<h2 class="h2">Edit Team</h2>

	<hr class="hr my-5" />

	<form class="form">
		<label class="label">
			<span>Team Name</span>
			<div class="input-group input-group-divider grid-cols-[1fr_auto]">
				<input
					class:input-error={teamNameError !== undefined}
					class="input"
					type="text"
					bind:value={teamNameInput}
					on:keypress={() => (teamNameError = undefined)}
					on:change={() => (teamNameError = undefined)}
					placeholder="eg. Green Dragons"
				/>
				<div class:input-error={(teamNameInput?.length ?? 0) > 64} class="input-group-shim">
					{teamNameInput?.length ?? 0}/64
				</div>
			</div>
			{#if teamNameError !== undefined}
				<span class="text-error-500">{teamNameError}</span>
			{/if}
		</label>
		<label>
			<span
				>Grouping <span
					aria-haspopup="dialog"
					use:popup={{
						event: 'hover',
						target: 'groupsPopup',
						placement: 'right'
					}}
				>
					<Fa class="inline" size="xs" icon={faCircleInfo} />
				</span></span
			>
			<div class="card arrow w-72 p-4 shadow-xl [&>*]:pointer-events-none" data-popup="groupsPopup">
				<div>
					<p>
						<strong>What are groups?</strong>
					</p>
					<p>
						On the app's home screen, you can create <u>labels</u> to group teams. These labels are available
						here, where you may add as many of them as you'd like to this team.
					</p>
				</div>
				<div class="bg-surface-100-800-token arrow" />
			</div>
			<div class="space-y-2">
				{#if groups !== undefined}
					{#each groups as group}
						<label class="flex items-center space-x-2">
							<input class="checkbox" type="checkbox" value={group.name} bind:group={tags} />
							<p>{group.name}</p>
						</label>
					{:else}
						<span
							>You have not created any labels. <a
								class="btn underline"
								href="/groups"
								on:click={parent.onClose}>Create a group here</a
							></span
						>
					{/each}
				{:else}
					<ProgressRadial />
				{/if}
			</div>
		</label>
	</form>

	<hr class="hr my-5" />

	<div class="flex flex-row-reverse">
		<button class="variant-filled btn" on:click={confirm}>Confirm</button>
		<button class="variant-outline btn mx-1" on:click={close}>Close</button>
	</div>
</div>
