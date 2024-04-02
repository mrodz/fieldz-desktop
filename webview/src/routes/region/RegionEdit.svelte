<script lang="ts">
	import type { CreateRegionInput, EditRegionInput, Region } from '$lib';
	import { ProgressRadial, getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	export let parent: any;

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	let region: Region | undefined;

	onMount(async () => {
		try {
			const id: number | undefined = $modalStore[0].meta?.id;

			if (id === undefined) {
				throw 'Modal parent did not give region id';
			}

			region = await invoke<Region>('load_region', { id });
			regionNameInput = region.title;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not load region',
				type: 'error'
			});
		}
	});

	let regionNameInput: string;
	let regionNameError: string | undefined;

	function close() {
		parent.onClose();
	}

	async function confirm() {
		if (regionNameInput === region!.title) {
			toastStore.trigger({
				message: "You didn't change the title",
				background: 'variant-filled-warning'
			});
			return;
		}

		const input = {
			id: $modalStore[0].meta!.id!,
			name: regionNameInput
		} satisfies EditRegionInput;

		try {
			const regionUpdated = await invoke<Region>('update_region', { input });

			region = regionUpdated;

			$modalStore[0].meta?.onUpdate(regionUpdated);

			close();
		} catch (e: any) {
			/*
			 * db\entity\src\lib.rs
			 */
			if (typeof e === 'object' && 'ValidationError' in e) {
				const error = e['ValidationError'];

				if (error === 'EmptyName') {
					regionNameError = 'Region name cannot be empty';
				} else if (typeof error === 'object' && 'NameTooLong' in error) {
					const nameTooLong = error['NameTooLong'];
					regionNameError = `Region name is ${nameTooLong?.len} characters which is larger than the max, 64`;
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
	{#if region === undefined}
		<ProgressRadial />
	{:else}
		<h2 class="h2">Editing Region</h2>

		<hr class="hr my-5" />

		<form class="form">
			<label class="label">
				<span>Region Name</span>
				<div class="input-group input-group-divider grid-cols-[1fr_auto]">
					<input
						class:input-error={regionNameError !== undefined}
						class="input"
						type="text"
						bind:value={regionNameInput}
						on:keypress={() => (regionNameError = undefined)}
						on:change={() => (regionNameError = undefined)}
					/>
					<div class:input-error={(regionNameInput?.length ?? 0) > 64} class="input-group-shim">
						{regionNameInput?.length ?? 0}/64
					</div>
				</div>
				{#if regionNameError !== undefined}
					<span class="text-error-500">{regionNameError}</span>
				{/if}
			</label>
		</form>

		<hr class="hr my-5" />
	{/if}

	<div class="flex flex-row-reverse">
		<button class="variant-filled btn" on:click={confirm}>Save Changes</button>
		<button class="variant-outline btn mx-1" on:click={close}>Close</button>
	</div>
</div>
