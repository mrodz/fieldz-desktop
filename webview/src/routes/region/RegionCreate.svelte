<script lang="ts">
	import type { CreateRegionInput, Region } from '$lib';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';

	export let parent: any;

	const modalStore = getModalStore();

	let regionNameInput: string | undefined;
	let regionNameError: string | undefined;

	function close() {
		parent.onClose();
	}

	async function confirm() {
		const input = {
			title: regionNameInput ?? ''
		} satisfies CreateRegionInput;

		try {
			const newRegion = await invoke<Region>('create_region', { input });

			$modalStore[0].meta?.onCreate(newRegion);

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
				<div class="input-group-shim {(regionNameInput?.length ?? 0) > 64 ? 'input-error' : ''}">
					{regionNameInput?.length ?? 0}/64
				</div>
			</div>
			{#if regionNameError !== undefined}
				<span class="text-error-500">{regionNameError}</span>
			{/if}
		</label>
	</form>

	<hr class="hr my-5" />

	<div class="flex flex-row-reverse">
		<button class="variant-filled btn" on:click={confirm}>Confirm</button>
		<button class="variant-outline btn mx-1" on:click={close}>Close</button>
	</div>
</div>
