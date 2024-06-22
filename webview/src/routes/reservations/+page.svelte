<script lang="ts">
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import Interaction from '@event-calendar/interaction';
	import { slide, fade } from 'svelte/transition';
	import {
		getModalStore,
		getToastStore,
		Accordion,
		AccordionItem,
		ProgressRadial,
		RadioGroup,
		RadioItem
	} from '@skeletonlabs/skeleton';
	import { onMount } from 'svelte';
	import { dialog, invoke } from '@tauri-apps/api';
	import {
		type CreateTimeSlotInput,
		type CalendarEvent,
		type Field,
		type MoveTimeSlotInput,
		type ReservationType,
		type TimeSlotExtension,
		type FieldSupportedConcurrencyInput,
		type UpdateReservationTypeConcurrencyForFieldInput,
		eventFromTimeSlot,
		MAX_GAMES_PER_FIELD_TYPE,
		MIN_GAMES_PER_FIELD_TYPE,
		type FieldConcurrency,
		TIME_SLOT_CREATION_MODAL_ENABLE,
		type Delta,
		type DateRange
	} from '$lib';
	import Fa from 'svelte-fa';
	import { faPaintRoller } from '@fortawesome/free-solid-svg-icons';

	const queryParams = new URLSearchParams(window.location.search);

	const fieldId = queryParams.get('fieldId');

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let rawEvents: TimeSlotExtension[] = [];

	let field: Field | undefined;

	let reservationTypes: ReservationType[] | undefined;

	let calendar: typeof Calendar;

	let activeScheduleType: ReservationType | undefined;

	let gamesPerFieldType: FieldConcurrency[] | undefined;

	let mismatches: Record<number, boolean> = {};

	onMount(async () => {
		try {
			if (fieldId === null || !Number.isInteger(Number(fieldId))) {
				dialog.message('Could not parse number from field id', {
					title: 'Error getting reservations',
					type: 'error'
				});
				return;
			}

			[field, reservationTypes] = await Promise.all([
				invoke<Field>('get_field', {
					fieldId: Number(fieldId)
				}),
				invoke<ReservationType[]>('get_reservation_types')
			]);

			/*
			 * Avoid fetching lots of data if the calendar won't render.
			 */
			if (reservationTypes.length !== 0) {
				rawEvents = await invoke<TimeSlotExtension[]>('get_time_slots', {
					fieldId: Number(fieldId)
				});
			} else {
				rawEvents = [];
			}

			const input = {
				reservation_type_ids: reservationTypes.map((t) => t.id),
				field_id: Number(fieldId)
			} satisfies FieldSupportedConcurrencyInput;

			activeScheduleType = reservationTypes.at(0);

			for (const event of rawEvents) {
				calendar.addEvent(eventFromTimeSlot(event));
			}

			gamesPerFieldType = await invoke<FieldConcurrency[]>('get_supported_concurrency_for_field', {
				input
			});

			for (const reservationType of reservationTypes) {
				const fieldConcurrency = gamesPerFieldType?.find(
					(fc) => fc.reservation_type_id === reservationType.id
				)!;

				mismatches[reservationType.id] =
					reservationType.default_sizing !== fieldConcurrency.concurrency;
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

	const dateStart = queryParams.get('d');

	async function createTimeSlot(input: CreateTimeSlotInput) {
		try {
			const newWindow: TimeSlotExtension = await invoke<TimeSlotExtension>('create_time_slot', {
				input
			});

			calendar.addEvent(eventFromTimeSlot(newWindow));
		} catch (err: any) {
			if (typeof err === 'object' && 'Overlap' in err) {
				toastStore.trigger({
					message: 'This would overlap with another time slot!',
					background: 'variant-filled-error',
					timeout: 1500
				});
			} else {
				dialog.message(JSON.stringify(err), {
					title: 'could not move event',
					type: 'error'
				});
			}
		}
	}

	let editMode: 'create' | 'select' = 'create';

	const options = {
		allDaySlot: false,
		view: 'timeGridWeek',
		firstDay: 1,
		editable: true,
		selectable: true,
		events: [],
		slotMinTime: '05:00:00',
		slotMaxTime: '24:00:00',
		unselectAuto: false,
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
					field_id: Number(fieldId),
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

			try {
				const input: MoveTimeSlotInput = {
					field_id: Number(fieldId),
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
		eventClick(e: { el: HTMLElement; event: CalendarEvent }) {
			deleteCalendarEventPrompt(e.event);
		},
		select(range: DateRange) {
			switch (editMode) {
				case 'create':
					dragToCreate(range);
					calendar.unselect();
					break;
				case 'select':
					dragToSelect(range);
					break;
			}
		},
		async dateClick(e: { date: Date }) {
			if (intendingToCopy) {
				modalStore.trigger({
					type: 'confirm',
					title: `You are about to paste ${selectionBuffer.length} time slot(s) at ${e.date}. Do you wish to continue?`,
					async response(r) {
						if (!r) return;

						try {
							const first = selectionBuffer[0].id;
							const last = selectionBuffer[selectionBuffer.length - 1].id;
							const newTimeSlots = await invoke<TimeSlotExtension[]>('copy_time_slots', {
								input: {
									src_start_id: Number(first),
									src_end_id: Number(last),
									dst_start: e.date.getTime()
								}
							});

							for (const timeSlot of newTimeSlots) {
								calendar.addEvent(eventFromTimeSlot(timeSlot));
							}

							toastStore.trigger({
								message: `Succesfully copied ${selectionBuffer.length} time slot(s)`,
								background: 'variant-filled-success'
							});

							onModeSwitch();
						} catch (err) {
							if (err !== null && typeof err === 'object' && 'Overlap' in err) {
								toastStore.trigger({
									message: 'This would overlap with another time slot! Try pasting elsewhere',
									background: 'variant-filled-error',
									timeout: 3500
								});
							
								// no `onModeSwitch` because we'll let the user try again
							} else {
								console.error(err);
								dialog.message(JSON.stringify(err), {
									title: 'could not copy events',
									type: 'error'
								});
								onModeSwitch();
							}
						}
					}
				});
			}
		}
	} as const;

	function dragToCreate(e: DateRange) {
		let diff: number = e.end.valueOf() - e.start.valueOf();
		let diffInHours = diff / 1000 / 60 / 60; // Convert milliseconds to hours

		let hours = Math.floor(diffInHours);
		let minutes = Math.floor((diffInHours - hours) * 60);

		const input = {
			start: e.start.valueOf(),
			end: e.end.valueOf(),
			reservation_type_id: activeScheduleType!.id,
			field_id: Number(fieldId)
		};

		if (TIME_SLOT_CREATION_MODAL_ENABLE) {
			modalStore.trigger({
				type: 'confirm',
				title: `New Reservation (${hours}:${minutes < 10 ? '0' + minutes : minutes}h duration)`,
				body: `From ${e.start} to ${e.end}`,
				buttonTextConfirm: 'Yes!',
				buttonTextCancel: 'No, go back',
				async response(r: boolean) {
					if (r) {
						createTimeSlot(input);
					}
				}
			});
		} else {
			createTimeSlot(input);
		}
	}

	let selectionBuffer: CalendarEvent[] = [];
	let triggerId: string | undefined;
	let copyTriggerId: string | undefined;

	function dragToSelect(e: DateRange) {
		if (triggerId !== undefined) {
			toastStore.close(triggerId);
			triggerId = undefined;
		}
		selectionBuffer = [];

		const unsorted = [];

		for (const event of calendar.getEvents() as CalendarEvent[]) {
			if (
				(event.start > e.start && event.start < e.end) ||
				(event.end > e.start && event.end < e.end)
			) {
				unsorted.push(event);
			}
		}

		selectionBuffer = unsorted.sort((a, b) => a.start.getTime() - b.start.getTime());

		triggerId = toastStore.trigger({
			message: `Selected ${selectionBuffer.length} event${selectionBuffer.length === 1 ? '' : 's'}`,
			autohide: false,
			callback(response) {
				if (response.status === 'closed' && triggerId === response.id) {
					onModeSwitch(false);
				}
			}
		});
	}

	function onModeSwitch(closeToast: boolean | undefined = true) {
		calendar.unselect();
		selectionBuffer = [];
		intendingToCopy = false;
		// the parameter is necessary to prevent a stack overflow if a toast tries to close itself
		if (closeToast && triggerId !== undefined) {
			toastStore.close(triggerId);
			triggerId = undefined;
		}
		if (copyTriggerId !== undefined) {
			toastStore.close(copyTriggerId);
			copyTriggerId = undefined;
		}
	}

	let intendingToCopy = false;

	function onIntentToCopy() {
		if (selectionBuffer.length === 0) {
			toastStore.trigger({
				message: 'There are no time slots in your selection',
				background: 'variant-filled-error'
			});
			return;
		}

		if (intendingToCopy) {
			toastStore.trigger({
				message: `You are already poised to copy ${selectionBuffer.length} time slot(s)`,
				background: 'variant-filled-error'
			});
			return;
		}

		intendingToCopy = true;

		copyTriggerId = toastStore.trigger({
			message: 'Copy your selection by clicking anywhere to insert',
			autohide: false,
			callback(response) {
				if (response.status === 'closed') {
					intendingToCopy = false;
				}
			}
		});
	}

	async function signalCustomConcurrencyUpdate(fc: FieldConcurrency) {
		try {
			const input = {
				field_id: Number(fieldId),
				reservation_type_id: fc.reservation_type_id,
				new_concurrency: fc.concurrency
			} satisfies UpdateReservationTypeConcurrencyForFieldInput;

			await invoke('update_reservation_type_concurrency_for_field', {
				input
			});

			const isDefaultSize =
				reservationTypes?.find((rt) => rt.id === fc.reservation_type_id)?.default_sizing !==
				fc.concurrency;
			mismatches[fc.reservation_type_id] = isDefaultSize;

			// signal UI refresh
			gamesPerFieldType = gamesPerFieldType;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not set custom field type concurrency',
				type: 'error'
			});
		}
	}

	async function increaseCount(typeId: number) {
		const thisType = gamesPerFieldType!.find((fc) => fc.reservation_type_id === typeId);

		if (thisType!.concurrency < MAX_GAMES_PER_FIELD_TYPE) {
			// eagerly re-render
			thisType!.concurrency++;
			gamesPerFieldType = gamesPerFieldType;

			await signalCustomConcurrencyUpdate(thisType!);
		}
	}

	async function decreaseCount(typeId: number) {
		const thisType = gamesPerFieldType!.find((fc) => fc.reservation_type_id === typeId);

		if (thisType!.concurrency > MIN_GAMES_PER_FIELD_TYPE) {
			// eagerly re-render
			thisType!.concurrency--;
			gamesPerFieldType = gamesPerFieldType;

			await signalCustomConcurrencyUpdate(thisType!);
		}
	}

	async function deleteCalendarEventPrompt(event: CalendarEvent) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Delete time slot',
			body: `Start: ${event.start}, End: ${event.end}`,
			buttonTextConfirm: 'Delete',
			async response(r: boolean) {
				if (r) {
					try {
						await invoke('delete_time_slot', { id: Number(event.id) });
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

	function batchDelete() {
		if (selectionBuffer.length === 0) {
			toastStore.trigger({
				message: 'There are no time slots in this selection to delete',
				background: 'variant-filled-error'
			});
			return;
		}

		const first = selectionBuffer[0];

		if (selectionBuffer.length === 1) {
			deleteCalendarEventPrompt(first);
			onModeSwitch();
			return;
		}

		const last = selectionBuffer[selectionBuffer.length - 1];

		modalStore.trigger({
			type: 'confirm',
			title: "You're about to delete multiple time slots",
			body: `Please confirm that you would like to delete ${selectionBuffer.length} time slots. This means every time slot from ${first.start} to ${last.end} will be <b>PERMANENTLY DELETED</b>. Only proceed if you are sure you'd like to delete these time slots.`,
			async response(r) {
				if (r) {
					try {
						await invoke('delete_time_slots_batched', {
							startId: Number(first.id),
							endId: Number(last.id)
						});
						for (const event of selectionBuffer) {
							calendar.removeEventById(event.id);
						}
						toastStore.trigger({
							message: `Permanently deleted ${selectionBuffer.length} time slots`,
							background: 'variant-filled-success'
						});
					} catch (e) {
						dialog.message(JSON.stringify(e), {
							title: 'Could not delete time slots',
							type: 'error'
						});
					} finally {
						onModeSwitch();
					}
				}
			}
		});
	}
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp; Back</button>

	{#if field === undefined}
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4">Availability: {field?.name}</h1>
	{/if}

	{#if activeScheduleType !== undefined}
		<Accordion>
			<AccordionItem>
				<svelte:fragment slot="summary">
					Click to expand help and accessibility information
				</svelte:fragment>
				<svelte:fragment slot="content">
					<section
						class="my-4 grid grid-cols-1 justify-items-center sm:gap-2 md:grid-cols-2 md:gap-4 lg:grid-cols-4 lg:gap-8 [&>div]:w-4/5 md:[&>div]:w-full"
					>
						<div class="card max-w-md p-4 text-center">
							<strong>Click and drag</strong> over empty space to
							{#if editMode === 'create'}
								create a time slot
							{:else}
								select many slots
							{/if}
						</div>
						<div class="card max-w-md p-4 text-center">
							<strong>Click and drag</strong> an event to move it or resize it
						</div>
						<div class="card max-w-md p-4 text-center">
							<strong>Click</strong> an event to delete it
						</div>
						<div class="card max-w-md p-4 text-center">
							<strong>Select</strong> a field type to switch between reservation sizes
						</div>
					</section>
				</svelte:fragment>
			</AccordionItem>
		</Accordion>
	{/if}

	<hr class="hr my-5" />

	{#if reservationTypes === undefined}
		<ProgressRadial />
	{:else if reservationTypes.length === 0}
		<div class="card bg-warning-500 m-4 mx-auto p-8 text-center">
			You must create at least one reservation type before you can craft a schedule. You can do so <a
				class="btn underline"
				href="/field-types">here</a
			>
		</div>
	{:else}
		<section>
			<div class="grid grid-cols-2 gap-8 xl:grid-cols-3">
				{#each reservationTypes as reservationType}
					{@const concurrency = gamesPerFieldType?.find(
						(fc) => fc.reservation_type_id === reservationType.id
					)?.concurrency}

					<div class="flex flex-col">
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
						<div class="card mx-auto grid grid-cols-[1fr_auto_1fr]">
							<button
								class="-x-variant-ghost btn-icon btn-icon-sm mr-auto"
								on:click={() => decreaseCount(reservationType.id)}>-</button
							>
							<div class="mx-2 text-center align-middle leading-loose">
								<span class="inline">
									{concurrency}
								</span>
								{#if concurrency !== reservationType.default_sizing}
									<span class="inline" style="color: red">*</span>
								{/if}
							</div>
							<button
								class="-x-variant-ghost btn-icon btn-icon-sm ml-auto"
								on:click={() => increaseCount(reservationType.id)}>+</button
							>
						</div>
					</div>
				{/each}
			</div>

			<div class="mt-4 flex items-center">
				<h2 class="h3 min-w-36">
					Using type:
					<strong style="color: {activeScheduleType?.color}">
						{activeScheduleType?.name}
					</strong>
				</h2>
				<div class="grow" />
				{#if Object.values(mismatches).some((isDefault) => isDefault)}
					<span in:fade out:fade>
						<span style="color: red">*</span> = custom reservation type/field partitioning in place.
					</span>
				{/if}
			</div>
		</section>
	{/if}

	<hr class="hr my-5" />

	<div class="block grid grid-cols-[auto_1fr] items-center">
		<RadioGroup active="variant-filled-primary" hover="hover:variant-soft-primary">
			<RadioItem
				on:click={() => onModeSwitch()}
				bind:group={editMode}
				name="editModePicker"
				value="create">Create Mode</RadioItem
			>
			<RadioItem
				on:click={() => onModeSwitch()}
				bind:group={editMode}
				name="editModePicker"
				value="select">Select Mode</RadioItem
			>
		</RadioGroup>

		{#if editMode === 'select'}
			<div class="ml-4 flex items-center gap-2" in:slide={{ axis: 'y' }} out:slide={{ axis: 'y' }}>
				<header>
					Selection: {selectionBuffer.length} time slot{selectionBuffer.length === 1 ? '' : 's'}
				</header>
				<div>
					<button class="variant-filled btn" on:click={batchDelete}>Delete Selected</button>
					<button
						class="variant-filled btn"
						on:click={() => {
							toastStore.trigger({
								message: 'Cleared selection',
								background: 'variant-filled-success'
							});
							onModeSwitch();
						}}>Clear Selected</button
					>
					<button class="variant-filled btn" on:click={onIntentToCopy}
						>{intendingToCopy ? 'Paste' : 'Copy'} Selected</button
					>
				</div>
			</div>
		{/if}
	</div>

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
