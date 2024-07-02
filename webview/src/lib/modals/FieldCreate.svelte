<script lang="ts">
	import type { CreateFieldInput, Field } from '$lib';
	import { getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';

	export let parent: any;

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	let fieldNameInput: string | undefined;
	let fieldNameError: string | undefined;

	function close() {
		parent.onClose();
	}

	async function confirm() {
		const payload: CreateFieldInput = {
			name: fieldNameInput ?? '',
			region_id: $modalStore[0].meta.region.id
		};

		try {
			const newField: Field = await invoke('create_field', {
				input: payload
			});

			toastStore.trigger({
				message: `Created new field: "${newField.name}"`,
				background: 'variant-filled-success'
			});

			$modalStore[0].meta?.onCreate(newField);

			close();
		} catch (e: any) {
			/*
			 * db\entity\src\lib.rs
			 */
			if (typeof e === 'object' && 'ValidationError' in e) {
				const error = e['ValidationError'];

				if (error === 'EmptyName') {
					fieldNameError = 'Field name cannot be empty';
				} else if (typeof error === 'object' && 'NameTooLong' in error) {
					const nameTooLong = error['NameTooLong'];
					fieldNameError = `Field name is ${nameTooLong?.len} characters which is larger than the max, 64`;
				} else {
					// unknown validation error!
					dialog.message(JSON.stringify(e), {
						title: 'Could not create field',
						type: 'error'
					});
				}
			} else {
				dialog.message(JSON.stringify(e), {
					title: 'Could not create field',
					type: 'error'
				});
			}
		}
	}
</script>

<div class="card w-modal p-5">
	<form class="form">
		<label class="label">
			<span>Field Name</span>
			<div class="input-group input-group-divider grid-cols-[1fr_auto]">
				<input
					class:input-error={fieldNameError !== undefined}
					class="input"
					type="text"
					bind:value={fieldNameInput}
					on:keypress={() => (fieldNameError = undefined)}
					on:change={() => (fieldNameError = undefined)}
					placeholder="eg. Rolling Hills Sports Complex - North"
				/>
				<div class:input-error={(fieldNameInput?.length ?? 0) > 64} class="input-group-shim">
					{fieldNameInput?.length ?? 0}/64
				</div>
			</div>
			{#if fieldNameError !== undefined}
				<span class="text-error-500">{fieldNameError}</span>
			{/if}
		</label>
	</form>

	<hr class="hr my-5" />

	<div class="flex flex-row-reverse">
		<button class="variant-filled btn" on:click={confirm}>Confirm</button>
		<button class="variant-outline btn mx-1" on:click={close}>Close</button>
	</div>
</div>
