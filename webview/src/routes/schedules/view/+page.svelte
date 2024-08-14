<script lang="ts" context="module">
	import { writable } from 'svelte/store';
	let compact = writable(false);
</script>

<script lang="ts">
	import type {
		Schedule,
		TeamExtension,
		ScheduleGame,
		CalendarEvent,
		Delta,
		MoveTimeSlotInput
	} from '$lib';
	import { eventFromGame, formatDatePretty } from '$lib';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import {
		ProgressRadial,
		SlideToggle,
		getModalStore,
		getToastStore
	} from '@skeletonlabs/skeleton';
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import List from '@event-calendar/list';

	import Interaction from '@event-calendar/interaction';

	const queryParams = new URLSearchParams(window.location.search);
	const idParam = queryParams.get('id');

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	const teams: Map<number, TeamExtension> = new Map();

	async function getTeam(id: number): Promise<TeamExtension> {
		const maybeTeam = teams.get(id);

		if (maybeTeam !== undefined) return maybeTeam;

		return invoke<TeamExtension>('get_team', { id });
	}

	if (idParam === null || idParam === '') {
		dialog.message(`Received a bad query parameter for 'id' (got: ${JSON.stringify(idParam)})`);
		history.back();
	}

	const id: number = Number(idParam);

	if (!Number.isInteger(id)) {
		dialog.message(
			`Received a bad query parameter for 'id' (got non-int: ${JSON.stringify(idParam)})`
		);
		history.back();
	}

	let schedule: Promise<[Schedule, ScheduleGame[]]> | undefined;

	let calendar: typeof Calendar;
	const plugins = [TimeGrid, List, Interaction] as const;

	let swapping: CalendarEvent | undefined;

	const options = {
		allDaySlot: false,
		view: $compact ? 'listWeek' : 'timeGridWeek',
		firstDay: 1,
		eventStartEditable: false,
		eventDurationEditable: false,
		selectable: false,
		events: [],
		slotMinTime: '05:00:00',
		slotMaxTime: '24:00:00',
		async eventDrop(e: {
			oldEvent: CalendarEvent;
			event: CalendarEvent;
			delta: Delta;
			revert: () => void;
		}) {
			const delta = e.delta;
			let canSkip = true;

			for (const key in delta) {
				const element = delta[key as keyof typeof delta];
				if (typeof element === 'number' && element !== 0) {
					canSkip = false;
				}
			}

			if (canSkip) {
				return;
			}

			let schedulePart = (await schedule!)[0];

			try {
				const input: MoveTimeSlotInput = {
					schedule_id: Number(schedulePart.id),
					id: Number(e.event.id),
					new_start: e.event.start.valueOf(),
					new_end: e.event.end.valueOf()
				};

				await invoke('move_time_slot', { input });
			} catch (err: any) {
				if ('Overlap' in err) {
					e.revert();
					toastStore.trigger({
						message: 'This would overlap with another time slot!',
						background: 'variant-filled-error',
						timeout: 1500
					});
				} else {
					console.error(err);
					dialog.message(JSON.stringify(err), {
						title: 'could not move event',
						type: 'error'
					});
				}
			}
		},
		async eventResize(e: {
			oldEvent: CalendarEvent;
			event: CalendarEvent;
			endDelta: Delta;
			revert: () => void;
		}) {
			const delta = e.endDelta;
			let canSkip = true;

			for (const key in delta) {
				const element = delta[key as keyof typeof delta];
				if (typeof element === 'number' && element !== 0) {
					canSkip = false;
				}
			}

			if (canSkip) {
				return;
			}

			let schedulePart = (await schedule!)[0];

			try {
				const input: MoveTimeSlotInput = {
					schedule_id: Number(schedulePart.id),
					id: Number(e.event.id),
					new_start: e.event.start.valueOf(),
					new_end: e.event.end.valueOf()
				};

				await invoke('move_time_slot', { input });
			} catch (err: any) {
				if ('Overlap' in err) {
					e.revert();
					toastStore.trigger({
						message: 'This would overlap with another time slot!',
						background: 'variant-filled-error',
						timeout: 1500
					});
				} else {
					console.error(err);
					dialog.message(JSON.stringify(err), {
						title: 'could not move event',
						type: 'error'
					});
				}
			}
		},
		async eventClick(e: { el: HTMLElement; event: CalendarEvent }) {
			if (swapping !== undefined) {
				try {
					const ok = await invoke('swap_schedule_games', {
						a: Number(swapping.id),
						b: Number(e.event.id)
					});

					if (ok) {
						const swappingClone = {
							...swapping,
							start: new Date(e.event.start),
							end: new Date(e.event.end)
						} satisfies CalendarEvent;

						const eClone = {
							...e.event,
							start: new Date(swapping.start),
							end: new Date(swapping.end)
						} satisfies CalendarEvent;

						calendar.updateEvent(swappingClone);
						calendar.updateEvent(eClone);

						toastStore.trigger({
							message: `Swapped "${swapping.title}" with "${e.event.title}"	`,
							background: 'variant-filled-success'
						});
					} else {
						console.warn('Could not swap time slots because the ids were incorrect');
					}
				} catch (err) {
					console.error(err);
					dialog.message(JSON.stringify(err), {
						title: 'could not swap events',
						type: 'error'
					});
				} finally {
					swapping = undefined;
					return;
				}
			}

			const [schedulePart, games] = await schedule!;
			const game = games.find((game) => game.id === Number(e.event.id));

			if (game === undefined) {
				toastStore.trigger({
					message: 'You clicked on a game that does not exist',
					background: 'variant-filled-error'
				});
				return;
			}

			modalStore.trigger({
				type: 'component',
				component: 'scheduleGameEdit',
				meta: {
					game,
					schedule: schedulePart,
					event: e.event,
					onDelete: deleteCalendarEventPrompt,
					getTeam,
					onSwap: () => {
						swapping = e.event;
					}
				}
			});
		}
	};

	$: calendar?.setOption?.('view', $compact ? 'listWeek' : 'timeGridWeek');

	$: if (swapping !== undefined) {
		calendar.updateEvent({
			...swapping,
			backgroundColor: 'orange'
		} satisfies CalendarEvent);
	}

	function cancelMove() {
		if (swapping === undefined) return;

		calendar.updateEvent(swapping);

		swapping = undefined;
	}

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

	$: calendar?.setOption?.('editable', editMode);

	async function deleteCalendarEventPrompt(event: CalendarEvent) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Delete time slot',
			body: `Start: ${event.start}, End: ${event.end}`,
			buttonTextConfirm: 'Delete',
			async response(r: boolean) {
				if (r) {
					try {
						let schedulePart = (await schedule!)[0];

						await invoke('delete_time_slot', {
							id: Number(event.id),
							scheduleId: Number(schedulePart.id)
						});
						calendar.removeEventById(event.id);
					} catch (e) {
						dialog.message(JSON.stringify(e), {
							title: 'Could not delete time slot',
							type: 'error'
						});
					}
				}
			}
		});
	}
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
					Switch to {#if $compact}
						Calendar View
					{:else}
						Compact View
					{/if}
				</SlideToggle>
				<SlideToggle name="slider-label" bind:checked={editMode}>Editable</SlideToggle>
			</div>
		{/await}
	{/if}
	{#if swapping !== undefined}
		<div
			class="ml-4 flex items-center gap-2 bg-green-200 p-4"
			in:slide={{ axis: 'y' }}
			out:slide={{ axis: 'y' }}
		>
			<div>
				Selected: {swapping.title} @ {formatDatePretty(swapping.start)}
			</div>
			<div>
				Click on another reservation to swap it, or <button
					on:click={cancelMove}
					class="variant-filled btn">Cancel</button
				>
			</div>
		</div>
	{/if}

	<div>
		<Calendar bind:this={calendar} {plugins} {options} />
	</div>
</main>
