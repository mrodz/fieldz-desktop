<script lang="ts">
	import type { Schedule } from '$lib';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { ProgressRadial, SlideToggle } from '@skeletonlabs/skeleton';
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import List from '@event-calendar/list';

	import Interaction from '@event-calendar/interaction';

	const queryParams = new URLSearchParams(window.location.search);
	const idParam = queryParams.get('id');

	if (idParam === null || idParam === '') {
		dialog.message(`Recieved a bad query parameter for 'id' (got: ${JSON.stringify(idParam)})`);
		history.back();
	}

	const id: number = Number(idParam);

	if (!Number.isInteger(id)) {
		dialog.message(
			`Recieved a bad query parameter for 'id' (got non-int: ${JSON.stringify(idParam)})`
		);
		history.back();
	}

	let schedule: Promise<Schedule> | undefined;

	let calendar: typeof Calendar;
	const plugins = [TimeGrid, List, Interaction] as const;

	let compact: boolean = false;

	const options = {
		allDaySlot: false,
		view: compact ? 'listWeek' : 'timeGridWeek',
		firstDay: 1,
		editable: true,
		selectable: true,
		events: [],
		slotMinTime: '05:00:00',
		slotMaxTime: '24:00:00'
	};

	$: if (calendar !== undefined) {
		calendar?.setOption('view', compact ? 'listWeek' : 'timeGridWeek');
	}

	onMount(() => {
		schedule = invoke<Schedule>('get_schedule', { id });
		

		schedule.then((schedule) => {
			calendar.setOption('events', [/* todo! */])
		});
	});
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	{#if schedule === undefined}
		<h1 class="h2">Schedule loading...</h1>
		<button class="variant-filled btn my-4" on:click={() => history.back()}>
			&laquo;&nbsp; Back
		</button>
		<ProgressRadial />
	{:else}
		{#await schedule}
			<h1 class="h2">Schedule loading...</h1>
			<button class="variant-filled btn my-4" on:click={() => history.back()}>
				&laquo;&nbsp; Back
			</button>
			<ProgressRadial />
		{:then schedule}
			<h1 class="h2">{schedule.name}</h1>
			<div class="my-4 flex items-center gap-2">
				<button class="variant-filled btn" on:click={() => history.back()}>
					&laquo;&nbsp; Back
				</button>
				<SlideToggle name="slider-label" bind:checked={compact}>
					Switch to {#if compact}
						Calendar View
					{:else}
						Compact View
					{/if}
				</SlideToggle>
			</div>
			<div>
				<Calendar bind:this={calendar} {plugins} {options} />
			</div>
		{/await}
	{/if}
</main>
