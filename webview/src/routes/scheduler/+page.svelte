<script lang="ts">
	import { slide } from 'svelte/transition';
	import { onMount } from 'svelte';
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import List from '@event-calendar/list';
	import {
		type ListReservationsBetweenInput,
		type TimeSlot,
		type Field,
		type CalendarEvent,
		type TeamExtension,
		type Region,
		eventFromTimeSlot
	} from '$lib';
	import {
		getModalStore,
		Accordion,
		AccordionItem,
		Paginator,
		SlideToggle,
		Table,
		type PaginationSettings
	} from '@skeletonlabs/skeleton';

	import { dialog, event, invoke } from '@tauri-apps/api';

	let modalStore = getModalStore();

	let calendar: typeof Calendar;

	const plugins = [TimeGrid, List] as const;

	let compact = false;

	const datesQueried: Map<string, TimeSlot[]> = new Map();
	const fieldsCache: Map<number, Field> = new Map();
	const regionCache: Map<number, Region> = new Map();

	async function loadField(fieldId: number): Promise<Field> {
		if (fieldsCache.has(fieldId)) {
			return fieldsCache.get(fieldId)!;
		}

		const field = await invoke<Field>('get_field', { fieldId });
		fieldsCache.set(fieldId, field);
		return field;
	}

	async function loadRegion(id: number): Promise<Region> {
		if (regionCache.has(id)) {
			return regionCache.get(id)!;
		}

		const region = await invoke<Region>('load_region', { id });
		regionCache.set(id, region);
		return region;
	}

	async function titleFromTimeSlot(input: TimeSlot): Promise<string | undefined> {
		try {
			const field = await loadField(input.field_id);
			const region = await loadRegion(field.region_owner);
			return `Region: ${region.title}\nField: ${field.name}`;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error getting field (field id: ${input.field_id})`,
				type: 'error'
			});
		}
	}

	const options = {
		allDaySlot: false,
		view: compact ? 'listWeek' : 'timeGridWeek',
		editable: true,
		selectable: true,
		events: [],
		async datesSet(e: { start: Date; end: Date }) {
			/*
			 * Normalize dates to get rid of minor differences in date window.
			 * Also, must set milliseconds to 1 because of edge case where an
			 * event that starts at midnight on the start of the week would
			 * otherwise be duplicated.
			 */
			e.start.setHours(0, 0, 0, 1);
			e.end.setHours(0, 0, 0, 1);

			const input: ListReservationsBetweenInput = {
				start: e.start.valueOf(),
				end: e.end.valueOf()
			};

			const inputString = JSON.stringify(input);

			if (datesQueried.has(inputString)) {
				return;
			}

			let newEvents: TimeSlot[];
			try {
				newEvents = await invoke<TimeSlot[]>('list_reservations_between', { input });
			} catch (e) {
				dialog.message(JSON.stringify(e), {
					title: `Error loading reservations ({})`,
					type: 'error'
				});
				newEvents = [];
			}

			for (const event of newEvents) {
				try {
					const asCalendarEvent: CalendarEvent = eventFromTimeSlot(
						event,
						(await titleFromTimeSlot(event)) ?? '#error'
					);
					calendar.addEvent(asCalendarEvent);
				} catch (e) {
					dialog.message(JSON.stringify(e), {
						title: `Error getting field (field id: ${event.field_id})`,
						type: 'error'
					});
				}
			}

			datesQueried.set(inputString, newEvents);
		},
		eventClick(e: { event: CalendarEvent }) {
			const clickedId = Number(e.event.id);

			// If the event was loaded, it was cached.
			const backingEvent = Array.from(datesQueried.values())
				.flatMap((x) => x)
				.find((event) => event.id === clickedId);

			if (backingEvent === undefined) {
				dialog.message('backing event = undefined', {
					title: `Error retrieving event with id ${clickedId}`,
					type: 'error'
				});
				return;
			}

			modalStore.trigger({
				type: 'confirm',
				title: 'View calendar',
				body: `<div>Event start: ${backingEvent.start}</div><div>Event end: ${backingEvent.end}</div><br/>Would you like to visit this event's source calendar?`,
				buttonTextConfirm: 'Visit Calendar',
				buttonTextCancel: 'Back',
				async response(r: boolean) {
					if (r) {
						document.location.href = `/reservations/${backingEvent.field_id}?d=${e.event.start.valueOf()}`;
					}
				}
			});
		}
	} as const;

	$: if (calendar !== undefined) {
		calendar?.setOption('view', compact ? 'listWeek' : 'timeGridWeek');

		/*
		 * Whenever `compact` changes, the calendar is dropped from the DOM
		 * and we must inject the previous state.
		 */
		Promise.all(
			Array.from(datesQueried)
				.flatMap((kv) => kv[1])
				.map(async (timeSlot) =>
					eventFromTimeSlot(timeSlot, (await titleFromTimeSlot(timeSlot)) ?? '#err')
				)
		).then((events) => {
			// the DOM node might have been dropped since the promise was polled.
			calendar?.setOption('events', events);
		});
	}

	let teams: TeamExtension[] | undefined = undefined;

	onMount(async () => {
		try {
			teams = await invoke<TeamExtension[]>('load_all_teams');
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error loading teams`,
				type: 'error'
			});
		}
	});

	let paginationSettings = {
		page: 0,
		limit: 5,
		size: 0,
		amounts: [1, 2, 5, 10, 50]
	} satisfies PaginationSettings;

	$: paginationSettings.size = teams?.length ?? 0;

	$: paginatedSource =
		teams?.slice(
			paginationSettings.page * paginationSettings.limit,
			paginationSettings.page * paginationSettings.limit + paginationSettings.limit
		) ?? [];
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Scheduler</h1>

	<section class="card m-4 p-4">
		<h2 class="h3">Input Parameters</h2>

		<Accordion class="my-4">
			<AccordionItem>
				<svelte:fragment slot="summary">Time Slots</svelte:fragment>
				<svelte:fragment slot="content">
					<div class="m-4 p-4 text-center">
						These are the time slots that you've created across your regions. They will each be
						candidates for scheduling.
					</div>

					<SlideToggle name="slider-label" class="mt-4" bind:checked={compact}>
						Switch to {#if compact}
							Calendar View
						{:else}
							Compact View
						{/if}
					</SlideToggle>

					<Calendar bind:this={calendar} {plugins} {options} />
				</svelte:fragment>
			</AccordionItem>
			<AccordionItem>
				<svelte:fragment slot="summary">Teams</svelte:fragment>
				<svelte:fragment slot="content">
					<div class="m-4 p-4 text-center">
						These are the teams that you've created across your regions. They will be scheduled
						according to the ruleset you define for a given schedule.
					</div>

					{#await Promise.all(paginatedSource.map(async (p) => {
							const region = await loadRegion(p.team.region_owner);
							return [region.title, p.team.name, p.tags
									.map((g) => `<span class="chip variant-filled-success">${g.name}</span>`)
									.join(' ')];
						})) then body}
						<Table
							source={{
								head: ['Region', 'Name', 'Tags'],
								body
							}}
						/>
					{/await}

					<Paginator bind:settings={paginationSettings} showPreviousNextButtons={true} />
				</svelte:fragment>
			</AccordionItem>
		</Accordion>
	</section>
</main>
