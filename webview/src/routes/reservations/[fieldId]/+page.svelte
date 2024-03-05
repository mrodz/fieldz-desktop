<script lang="ts">
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import Interaction from '@event-calendar/interaction';
	import { slide } from 'svelte/transition';
	import { getModalStore, getToastStore, ProgressRadial } from '@skeletonlabs/skeleton';
	import { onMount } from 'svelte';
	import { dialog, invoke } from '@tauri-apps/api';
	import type {
		TimeSlot,
		CreateTimeSlotInput,
		CalendarEvent,
		Field,
		MoveTimeSlotInput
	} from '$lib';

	export let data;

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let rawEvents: TimeSlot[] = [];
	// $: events = rawEvents.map(eventFromTimeSlot) ?? []

	let field: Field | undefined;

	function eventFromTimeSlot(input: TimeSlot): CalendarEvent {
		return {
			allDay: false,
			display: 'auto',
			id: String(input.id),
			resources: [],
			start: new Date(input.start),
			end: new Date(input.end)
		};
	}

	onMount(async () => {
		try {
			[rawEvents, field] = await Promise.all([
				invoke<TimeSlot[]>('get_time_slots', {
					fieldId: data.fieldId
				}),
				invoke<Field>('get_field', {
					fieldId: data.fieldId
				})
			]);

			for (let event of rawEvents) {
				calendar.addEvent(eventFromTimeSlot(event));
			}
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Error getting reservations',
				type: 'error'
			});
		}
	});

	let plugins = [TimeGrid, Interaction];

	let calendar: typeof Calendar;

	type DateRange = {
		start: Date;
		end: Date;
	};

	type Delta = {
		years: number;
		months: number;
		days: number;
		seconds: number;
		inWeeks: boolean;
	};

	let options = {
		allDaySlot: false,
		view: 'timeGridWeek',
		editable: true,
		selectable: true,
		events: [],
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
				console.info('No need to send event move action to backend');
				return;
			}

			console.info(e);

			try {
				const input: MoveTimeSlotInput = {
					field_id: data.fieldId,
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
						background: 'variant-filled-error'
					});
				} else {
					dialog.message(JSON.stringify(err), {
						title: 'could not move event',
						type: 'error'
					});
				}
			}
			console.info(e);
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
				console.info('No need to send event move action to backend');
				return;
			}

			console.info(e);

			try {
				const input: MoveTimeSlotInput = {
					field_id: data.fieldId,
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
						background: 'variant-filled-error'
					});
				} else {
					dialog.message(JSON.stringify(err), {
						title: 'could not move event',
						type: 'error'
					});
				}
			}
			console.info(e);
		},
		eventClick(e: { el: HTMLElement, event: CalendarEvent }) {
			modalStore.trigger({
				type: 'confirm',
				title: 'Delete time slot',
				body: `Start: ${e.event.start}, End: ${e.event.end}`,
				buttonTextConfirm: 'Delete',
				async response(r: boolean) {
					if (r) {
						try {
							await invoke('delete_time_slot', { id: Number(e.event.id) })
							calendar.removeEventById(e.event.id);
						} catch (e) {
							dialog.message(JSON.stringify(e), {
								title: 'Could not delete time slot',
								type: 'error',
							})
						}
					}
				},
			})
		},
		select(e: DateRange) {
			let diff: number = e.end.valueOf() - e.start.valueOf();
			let diffInHours = diff / 1000 / 60 / 60; // Convert milliseconds to hours

			let hours = Math.floor(diffInHours);
			let minutes = Math.floor((diffInHours - hours) * 60);

			modalStore.trigger({
				type: 'confirm',
				title: `New Reservation (${hours}:${minutes < 10 ? '0' + minutes : minutes}h duration)`,
				body: `From ${e.start} to ${e.end}`,
				buttonTextConfirm: 'Yes!',
				buttonTextCancel: 'No, go back',
				async response(r: boolean) {
					if (r) {
						try {
							const input: CreateTimeSlotInput = {
								start: e.start.valueOf(),
								end: e.end.valueOf(),
								field_id: data.fieldId
							};

							const newWindow: TimeSlot = await invoke<TimeSlot>('create_time_slot', { input });

							calendar.addEvent(eventFromTimeSlot(newWindow));
						} catch (err: any) {
							if ('Overlap' in err) {
								toastStore.trigger({
									message: 'This would overlap with another time slot!',
									background: 'variant-filled-error'
								});
							} else {
								dialog.message(JSON.stringify(err), {
									title: 'could not move event',
									type: 'error'
								});
							}
						}
					}
				}
			});
		},
		
	};
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp; Fields</button>

	{#if field === undefined}
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4">Availability: {field?.name}</h1>
	{/if}

	<section class="grid grid-cols-3 justify-items-center my-4">
		<div class="card sm:mx-2 md:mx-4 lg:mx-8 max-w-md p-4 text-center">
			<strong>Click and drag</strong> over empty space to create a time slot
		</div>
		<div class="card sm:mx-2 md:mx-4 lg:mx-8 max-w-md p-4 text-center">
			<strong>Click and drag</strong> an event to move it or resize it
		</div>
		<div class="card sm:mx-2 md:mx-4 lg:mx-8 max-w-md p-4 text-center">
			<strong>Click</strong> an event to delete it
		</div>
	</section>

	<Calendar bind:this={calendar} {plugins} {options} />
</main>
