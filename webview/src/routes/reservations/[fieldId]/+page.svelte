<script lang="ts">
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import Interaction from '@event-calendar/interaction';
	import { slide } from 'svelte/transition';
	import { getModalStore, getToastStore, ProgressRadial } from '@skeletonlabs/skeleton';
	import { onMount } from 'svelte';
	import { dialog, invoke } from '@tauri-apps/api';
	import {
		type TimeSlot,
		type CreateTimeSlotInput,
		type CalendarEvent,
		type Field,
		type MoveTimeSlotInput,
		type ReservationType,
		eventFromTimeSlot,
		type TimeSlotExtension
	} from '$lib';
	import Fa from 'svelte-fa';
	import { faPaintRoller } from '@fortawesome/free-solid-svg-icons';

	export let data;

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let rawEvents: TimeSlotExtension[] = [];

	let field: Field | undefined;

	let reservationTypes: ReservationType[] | undefined;

	let calendar: typeof Calendar;

	let activeScheduleType: ReservationType | undefined;

	onMount(async () => {
		try {
			[rawEvents, field, reservationTypes] = await Promise.all([
				invoke<TimeSlotExtension[]>('get_time_slots', {
					fieldId: data.fieldId
				}),
				invoke<Field>('get_field', {
					fieldId: data.fieldId
				}),
				invoke<ReservationType[]>('get_reservation_types')
			]);

			activeScheduleType = reservationTypes.at(0);

			for (let event of rawEvents) {
				calendar.addEvent(eventFromTimeSlot(event));
			}
		} catch (e) {
			console.error(e);
			dialog.message(JSON.stringify(e), {
				title: 'Error getting reservations',
				type: 'error'
			});
		}
	});

	const plugins = [TimeGrid, Interaction] as const;

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

	const queryParams = new URLSearchParams(window.location.search);
	const dateStart = queryParams.get('d');
	const options = {
		allDaySlot: false,
		view: 'timeGridWeek',
		editable: true,
		selectable: true,
		events: [],
		date: dateStart === null || isNaN(Number(dateStart)) ? new Date() : new Date(Number(dateStart)),
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
		},
		eventClick(e: { el: HTMLElement; event: CalendarEvent }) {
			modalStore.trigger({
				type: 'confirm',
				title: 'Delete time slot',
				body: `Start: ${e.event.start}, End: ${e.event.end}`,
				buttonTextConfirm: 'Delete',
				async response(r: boolean) {
					if (r) {
						try {
							await invoke('delete_time_slot', { id: Number(e.event.id) });
							calendar.removeEventById(e.event.id);
						} catch (e) {
							dialog.message(JSON.stringify(e), {
								title: 'Could not delete time slot',
								type: 'error'
							});
						}
					}
				}
			});
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
								reservation_type_id: activeScheduleType!.id,
								field_id: data.fieldId
							};

							const newWindow: TimeSlotExtension = await invoke<TimeSlotExtension>(
								'create_time_slot',
								{ input }
							);

							calendar.addEvent(eventFromTimeSlot(newWindow));
						} catch (err: any) {
							if (typeof err === 'object' && 'Overlap' in err) {
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
		}
	} as const;
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp; Back</button>

	{#if field === undefined}
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4">Availability: {field?.name}</h1>
	{/if}

	{#if activeScheduleType !== undefined}
		<section class="my-4 grid grid-cols-4 justify-items-center">
			<div class="card max-w-md p-4 text-center sm:mx-2 md:mx-4 lg:mx-8">
				<strong>Click and drag</strong> over empty space to create a time slot
			</div>
			<div class="card max-w-md p-4 text-center sm:mx-2 md:mx-4 lg:mx-8">
				<strong>Click and drag</strong> an event to move it or resize it
			</div>
			<div class="card max-w-md p-4 text-center sm:mx-2 md:mx-4 lg:mx-8">
				<strong>Click</strong> an event to delete it
			</div>
			<div class="card max-w-md p-4 text-center sm:mx-2 md:mx-4 lg:mx-8">
				<strong>Select</strong> a field type to switch between reservation sizes
			</div>
		</section>
	{/if}

	<hr class="hr my-5" />

	{#if reservationTypes === undefined}
		<ProgressRadial />
	{:else if reservationTypes.length === 0}
		<div class="card m-4 mx-auto bg-warning-500 p-8 text-center">
			You must create at least one reservation type before you can craft a schedule. You can do so <a
				class="btn underline"
				href="/field-types">here</a
			>
		</div>
	{:else}
		<section>
			<div class="grid grid-cols-3 gap-8">
				{#each reservationTypes as reservationType}
					<button
						class="btn block grid grid-cols-[auto_1fr]"
						disabled={activeScheduleType?.id === reservationType.id}
						on:click={() => (activeScheduleType = reservationType)}
					>
						<span>
							<Fa icon={faPaintRoller} size="lg" />
						</span>
						<div class="flex p-5" style="background-color: {reservationType.color}">
							{reservationType.name}
						</div>
					</button>
				{/each}
			</div>

			<h2 class="h3 mt-4">
				Using type:
				<strong style="color: {activeScheduleType?.color}">
					{activeScheduleType?.name}
				</strong>
			</h2>
		</section>
	{/if}

	<hr class="hr my-5" />

	<div class:hidden={activeScheduleType === undefined}>
		<Calendar bind:this={calendar} {plugins} {options} />
	</div>

	{#if activeScheduleType === undefined}
		A calendar will appear here once you've created a reservation type.
		<br />
		<i>In the meantime, here's a taco:</i>
		&#x1F32E;
	{/if}
</main>
