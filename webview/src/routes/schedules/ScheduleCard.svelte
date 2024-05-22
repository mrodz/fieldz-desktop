<script lang="ts">
	import type { Schedule } from '$lib';
	import { faCaretDown, faPencil, faTrash } from '@fortawesome/free-solid-svg-icons';
	import { getModalStore, getToastStore, popup, type PopupSettings } from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';
	import { dialog, invoke } from '@tauri-apps/api';
	import { goto } from '$app/navigation';

	export let schedule: Schedule;

	const modalStore = getModalStore();
	const toastStore = getToastStore();

	const dispatch = createEventDispatcher<{
		delete: Schedule;
		update: {
			prev: Schedule;
			new: Schedule;
		};
	}>();

	const DATE_OPTIONS: Intl.DateTimeFormatOptions = {
		hour: 'numeric',
		minute: 'numeric'
	};

	const target = `popupClick-schedule-${schedule.id}-${schedule.name}`;

	const popupClick: PopupSettings = {
		event: 'click',
		target,
		placement: 'bottom'
	};

	function onDelete() {
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm—This one is really important',
			body: `Deleting a schedule is PERMANENT! Are you sure you wish to proceed?<br/><br/><b>⚠️ DELETING A SCHEDULE WILL PERMANENTLY ERASE ALL CALENDAR EVENTS ASSOCIATED WITH THIS SCHEDULE</b><br/><br/>You will NOT be able to recover "${schedule.name}". Only proceed if you are sure this is what you want.`,
			async response(r) {
				if (r) {
					try {
						await invoke('delete_schedule', {
							id: schedule.id
						});

						dispatch('delete', schedule);
					} catch (e) {
						dialog.message(JSON.stringify(e), {
							title: 'Error',
							type: 'error'
						});
					}
				}
			}
		});
	}

	function onRename() {
		modalStore.trigger({
			type: 'component',
			component: 'scheduleEdit',
			meta: {
				schedule,
				onUpdate(updatedSchedule: Schedule) {
					toastStore.trigger({
						message: `Saved changes for "${updatedSchedule.name}"`,
						background: 'variant-filled-success'
					});

					dispatch('update', {
						prev: schedule,
						new: updatedSchedule
					});

					schedule = updatedSchedule;
				}
			}
		});
	}

	function onClick() {
		goto(`/schedules/view?id=${schedule.id}`);
	}
</script>

<button class="card card-hover m-4 grid grid-cols-[1fr_auto] p-4 text-left" on:click={onClick}>
	<div>
		<header class="card-header">
			<strong>
				{schedule.name}
			</strong>
		</header>
		<section class="p-4">
			<div>
				<div>
					Created: {new Date(schedule.created).toLocaleDateString('en-US', DATE_OPTIONS)}
				</div>
				<div>
					Last Edited: {new Date(schedule.last_edit).toLocaleDateString('en-US', DATE_OPTIONS)}
				</div>
			</div>
		</section>
	</div>
	<div class="mx-2 flex items-center">
		<button class="btn-icon hover:variant-outline" on:click|stopPropagation use:popup={popupClick}>
			<Fa icon={faCaretDown} />
		</button>
	</div>
</button>

<div class="card variant-filled-primary p-4" data-popup={target}>
	<ul class="list">
		<li class="select-none">
			<button class="btn" on:click={onRename}>
				<span class="badge"><Fa icon={faPencil} /></span>
				Rename
			</button>
		</li>
		<li class="select-none">
			<button class="btn" on:click={onDelete}>
				<span class="badge"><Fa icon={faTrash} /></span>
				Delete
			</button>
		</li>
	</ul>
	<div class="variant-filled-primary arrow" />
</div>
