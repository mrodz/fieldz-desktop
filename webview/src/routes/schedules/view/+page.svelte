<script lang="ts" context="module">
	import { writable } from 'svelte/store';
	let compact = writable(false);
</script>

<script lang="ts">
	import type { Schedule, TeamExtension, ScheduleGame } from '$lib';
	import { eventFromGame } from '$lib';
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

	const teams: Map<number, TeamExtension> = new Map();

	async function getTeam(id: number): Promise<TeamExtension> {
		const maybeTeam = teams.get(id);

		if (maybeTeam !== undefined) return maybeTeam;

		return invoke<TeamExtension>('get_team', { id });
	}

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

	let schedule: Promise<[Schedule, ScheduleGame[]]> | undefined;

	let calendar: typeof Calendar;
	const plugins = [TimeGrid, List, Interaction] as const;

	const options = {
		allDaySlot: false,
		view: $compact ? 'listWeek' : 'timeGridWeek',
		firstDay: 1,
		eventStartEditable: false,
		eventDurationEditable: false,
		selectable: false,
		events: [],
		slotMinTime: '05:00:00',
		slotMaxTime: '24:00:00'
	};

	$: calendar?.setOption?.('view', $compact ? 'listWeek' : 'timeGridWeek');
	// $: {
	// 	calendar?.setOption?.('editable', editMode);
	// 	console.log(calendar?.getOption?.('editable'));
	// }

	onMount(async () => {
		try {
			schedule = invoke<[Schedule, ScheduleGame[]]>('get_schedule_games', { scheduleId: id });

			if (calendar !== undefined) {
				const [_schedule, games] = await schedule;
				const events = games.map((game) => eventFromGame(game, getTeam));

				let firstDay: Date | undefined;

				for await (const event of events) {
					calendar.addEvent(event);

					if (firstDay === undefined) {
						firstDay = event.start;
					} else if (event.start.getDate() < firstDay.getDate()) {
						firstDay = event.start;
					}
				}

				if (firstDay !== undefined) {
					calendar.setOption('date', firstDay);
				}
			}
		} catch (e) {
			console.error(e);
			dialog.message(JSON.stringify(e), {
				title: 'Could not transform games into calendar events',
				type: 'error'
			});
		}
	});

	let editMode: boolean = false;
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
			<h1 class="h2">{schedule[0].name}</h1>
			<div class="my-4 flex items-center gap-2">
				<button class="variant-filled btn" on:click={() => history.back()}>
					&laquo;&nbsp; Back
				</button>
				<SlideToggle name="slider-label" bind:checked={$compact}>
					Switch to {#if compact}
						Calendar View
					{:else}
						Compact View
					{/if}
				</SlideToggle>
				<!-- <SlideToggle name="slider-label" bind:checked={editMode}>
					Editable
				</SlideToggle> -->
			</div>
		{/await}
	{/if}
	<div>
		<Calendar bind:this={calendar} {plugins} {options} />
	</div>
</main>
