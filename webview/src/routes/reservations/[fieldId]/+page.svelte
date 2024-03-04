<script lang="ts">
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import Interaction from '@event-calendar/interaction';
	import { slide } from 'svelte/transition';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import { onMount } from 'svelte';
	import { dialog, invoke } from '@tauri-apps/api';
	import type { TimeSlot } from '$lib';

	export let data;

	let events: TimeSlot[] = []

	onMount(async () => {
		try {
			events = await invoke<TimeSlot[]>('get_time_slots', {
				fieldId: Number(data.fieldId)
			});
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Error getting reservations',
				type: 'error'
			});
		}
	})

	let modalStore = getModalStore();

	let plugins = [TimeGrid, Interaction];
	let options = {
		allDaySlot: false,
		view: 'timeGridWeek',
		editable: true,
		selectable: true,
		events,
		select(e: { start: Date, end: Date }) {
			let diff: number = e.end.valueOf() - e.start.valueOf();
			let diffInHours = diff/1000/60/60; // Convert milliseconds to hours

			let hours = Math.floor(diffInHours)
			let minutes = Math.floor((diffInHours - hours) * 60);

			modalStore.trigger({
				type: 'confirm',
				title: `New Reservation (${hours}:${minutes < 10 ? '0' + minutes : minutes} duration)`,
				body: `From ${e.start} to ${e.end}`,
				buttonTextConfirm: 'Yes!',
				buttonTextCancel: 'No, go back',
				response: function (r) {
					alert("I haven't finished this yet.")

					if (r) {
					}
				}
			});
			// alert(JSON.stringify(e));
		},
		datesSet(e: any) {
			// alert(JSON.stringify(e));
		}
	};
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp; Fields</button>

	<Calendar {plugins} {options} />
</main>