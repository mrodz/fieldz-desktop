<script lang="ts">
	import type { EditScheduleInput, Schedule } from '$lib';
	import { getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';

	export let parent: any;

	const modalStore = getModalStore();

	let scheduleNameInput: string | undefined = $modalStore[0].meta.schedule.name;
	let scheduleNameError: string | undefined;

	function close() {
		parent.onClose();
	}

	async function confirm() {
		const payload: EditScheduleInput = {
			name: scheduleNameInput ?? '',
			id: $modalStore[0].meta.schedule.id
		};

		try {
			const newSchedule: Schedule = await invoke('update_schedule', {
				input: payload
			});

			$modalStore[0].meta?.onUpdate?.(newSchedule);

			close();
		} catch (e: any) {
			console.error(e);
			/*
			 * db\entity\src\lib.rs
			 */
			if (typeof e === 'object' && 'ValidationError' in e) {
				const error = e['ValidationError'];

				if (error === 'EmptyName') {
					scheduleNameError = 'Schedule name cannot be empty';
				} else if (typeof error === 'object' && 'NameTooLong' in error) {
					const nameTooLong = error['NameTooLong'];
					scheduleNameError = `Schedule name is ${nameTooLong?.len} characters which is larger than the max, 64`;
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
			<span>Schedule Name</span>
			<div class="input-group input-group-divider grid-cols-[1fr_auto]">
				<input
					class:input-error={scheduleNameError !== undefined}
					class="input"
					type="text"
					bind:value={scheduleNameInput}
					on:keypress={() => (scheduleNameError = undefined)}
					on:change={() => (scheduleNameError = undefined)}
					placeholder="eg. My Test Schedule #1"
				/>
				<div class:input-error={(scheduleNameInput?.length ?? 0) > 64} class="input-group-shim">
					{scheduleNameInput?.length ?? 0}/64
				</div>
			</div>
			{#if scheduleNameError !== undefined}
				<span class="text-error-500">{scheduleNameError}</span>
			{/if}
		</label>
	</form>

	<hr class="hr my-5" />

	<div class="flex flex-row-reverse">
		<button class="variant-filled btn" on:click={confirm}>Confirm</button>
		<button class="variant-outline btn mx-1" on:click={close}>Close</button>
	</div>
</div>
