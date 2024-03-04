<script lang="ts">
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import Interaction from '@event-calendar/interaction';
	import { slide } from 'svelte/transition';
	import { getModalStore } from '@skeletonlabs/skeleton';

	export let data;

	let modalStore = getModalStore();

	let plugins = [TimeGrid, Interaction];
	let options = {
		allDaySlot: false,
		view: 'timeGridWeek',
		editable: true,
		selectable: true,
		events: [
			// your list of events
		],
		select(e: { start: Date, end: Date }) {
			modalStore.trigger({
				type: 'confirm',
				title: 'Is this correct?',
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