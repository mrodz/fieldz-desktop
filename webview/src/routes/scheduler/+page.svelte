<script lang="ts" context="module">
	let schedulerWait = false;
</script>

<script lang="ts">
	import { slide, crossfade } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
	import { onMount } from 'svelte';
	import Calendar from '@event-calendar/core';
	import TimeGrid from '@event-calendar/time-grid';
	import List from '@event-calendar/list';
	import {
		type ListReservationsBetweenInput,
		type Field,
		type CalendarEvent,
		type TeamExtension,
		type Region,
		eventFromTimeSlot,
		type TeamGroup,
		type TargetExtension,
		type PreScheduleReport,
		type PreScheduleReportInput,
		type ReservationType,
		type ScheduledInput,
		type ScheduledOutput,
		type TimeSlotExtension,
		type FieldConcurrency,
		type UpdateTargetReservationTypeInput,
		regionalUnionSumTotal,
		isSupplyRequireEntryAccountedFor,

		SCHEDULE_CREATION_DELAY

	} from '$lib';
	import {
		getModalStore,
		getToastStore,
		Accordion,
		AccordionItem,
		Paginator,
		SlideToggle,
		Table,
		type PaginationSettings,
		ProgressRadial,
		RangeSlider,
		CodeBlock,
		TabGroup,
		TabAnchor
	} from '@skeletonlabs/skeleton';

	import { dialog, event, invoke } from '@tauri-apps/api';
	import Target from './Target.svelte';
	import ScheduleErrorReport from './ScheduleErrorReport.svelte';
	import ReportTable from './ReportTable.svelte';

	import authStore from '$lib/authStore';
	import { goto } from '$app/navigation';
	import { getAuth } from 'firebase/auth';

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let calendar: typeof Calendar;

	const plugins = [TimeGrid, List] as const;

	let compact = false;

	const datesQueried: Map<string, TimeSlotExtension[]> = new Map();
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

	async function titleFromTimeSlot(input: TimeSlotExtension): Promise<string | undefined> {
		try {
			const field = await loadField(input.time_slot.field_id);
			const region = await loadRegion(field.region_owner);
			return `Region: ${region.title}\nField: ${field.name}`;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error getting field (field id: ${input.time_slot.field_id})`,
				type: 'error'
			});
		}
	}

	const options = {
		allDaySlot: false,
		view: compact ? 'listWeek' : 'timeGridWeek',
		firstDay: 1,
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

			let newEvents: TimeSlotExtension[];
			try {
				newEvents = await invoke<TimeSlotExtension[]>('list_reservations_between', { input });
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
						title: `Error getting field (field id: ${event.time_slot.field_id})`,
						type: 'error'
					});
				}
			}

			datesQueried.set(inputString, newEvents);
		},
		async eventClick(e: { event: CalendarEvent }) {
			const clickedId = Number(e.event.id);

			// If the event was loaded, it was cached.
			const backingEvent = Array.from(datesQueried.values())
				.flatMap((x) => x)
				.find((event) => event.time_slot.id === clickedId);

			if (backingEvent === undefined) {
				dialog.message('backing event = undefined', {
					title: `Error retrieving event with id ${clickedId}`,
					type: 'error'
				});
				return;
			}

			const field = await loadField(backingEvent.time_slot.field_id);
			const region = await loadRegion(field.region_owner);

			modalStore.trigger({
				type: 'confirm',
				title: 'View calendar',
				body: `<div><strong>Region:&nbsp;</strong>${region.title}</br><strong>Field:&nbsp;</strong>${field.name}<br/><br/>Event start: ${backingEvent.time_slot.start}</div><div>Event end: ${backingEvent.time_slot.end}</div><br/>Would you like to visit this event's source calendar?`,
				buttonTextConfirm: 'Visit Calendar',
				buttonTextCancel: 'Back',
				async response(r: boolean) {
					if (r) {
						document.location.href = `/reservations?fieldId=${backingEvent.time_slot.field_id}&d=${e.event.start.valueOf()}`;
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

	let teams: TeamExtension[] | undefined;
	let reservationTypes: ReservationType[] | undefined;
	let groups: TeamGroup[] | undefined;
	let targets: TargetExtension[] | undefined;
	let customResTypeSizePerField: FieldConcurrency[] | undefined;

	onMount(async () => {
		try {
			[teams, groups, targets, reservationTypes, customResTypeSizePerField] = await Promise.all([
				invoke<TeamExtension[]>('load_all_teams'),
				invoke<TeamGroup[]>('get_groups'),
				invoke<TargetExtension[]>('get_targets'),
				invoke<ReservationType[]>('get_reservation_types'),
				invoke<FieldConcurrency[]>('get_non_default_reservation_type_concurrency_associations')
			]);
			await generateReport();
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error loading scheduler page`,
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

	let reportTimer: NodeJS.Timeout | undefined;
	let normalSeasonReport: PreScheduleReport | undefined;
	let postSeasonReport: PreScheduleReport | undefined;

	let willSendReport = false;

	async function generateReport() {
		try {
			const normalSeasonInput = {
				matches_to_play: normalSeasonGamesToPlay,
				interregional: normalSeasonInter
			} satisfies PreScheduleReportInput;

			normalSeasonReport = await invoke<PreScheduleReport>('generate_pre_schedule_report', {
				input: normalSeasonInput
			});

			const postSeasonInput = {
				matches_to_play: postSeasonGamesToPlay,
				interregional: postSeasonInter
			} satisfies PreScheduleReportInput;

			postSeasonReport = await invoke<PreScheduleReport>('generate_pre_schedule_report', {
				input: postSeasonInput
			});
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error generating pre-schedule report`,
				type: 'error'
			});
		}
	}

	function updateTargets() {
		clearTimeout(reportTimer);
		willSendReport = true;
		reportTimer = setTimeout(async () => {
			await generateReport();
			willSendReport = false;
		}, 1_000);
	}

	async function createTarget() {
		try {
			const target = await invoke<TargetExtension>('create_target');
			targets!.push(target);
			targets = targets;
			updateTargets();
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error creating target`,
				type: 'error'
			});
		}
	}

	async function deleteTarget(target: TargetExtension, index: number) {
		try {
			await invoke('delete_target', { id: target.target.id });
			targets!.splice(index, 1);
			targets = targets;
			updateTargets();
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error deleting target`,
				type: 'error'
			});
		}
	}

	async function targetAddGroup(target: TargetExtension, group: TeamGroup) {
		try {
			await invoke('target_add_group', { targetId: target.target.id, groupId: group.id });
			updateTargets();
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error adding group to target`,
				type: 'error'
			});
		}
	}

	async function targetDeleteGroup(target: TargetExtension, group: TeamGroup) {
		try {
			await invoke('target_delete_group', { targetId: target.target.id, groupId: group.id });
			updateTargets();
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: `Error removing group from target`,
				type: 'error'
			});
		}
	}

	async function modifyReservationType(
		target: TargetExtension,
		newReservationType: ReservationType | 'unset' | '*'
	) {
		try {
			// null when the user does not click any input (aka. on first render)
			if (newReservationType !== 'unset') {
				const input = {
					target_id: target.target.id,
					new_reservation_type_id: newReservationType === '*' ? undefined : newReservationType.id
				} satisfies UpdateTargetReservationTypeInput;

				await invoke('update_target_reservation_type', { input });
				updateTargets();
			}
		} catch (e) {
			console.error(e);
			dialog.message(JSON.stringify(e), {
				title: `Error updating this target's reservation type`,
				type: 'error'
			});
		}
	}

	const [send, receive] = crossfade({
		duration: 250,
		easing: quintOut
	});

	const key = Symbol('key for crossfade animation');

	function isTargetOk(report: PreScheduleReport, target: TargetExtension): boolean {
		console.log(report);
		console.log(target);

		const isDuplicate = report.target_has_duplicates.includes(target.target.id);

		if (isDuplicate) {
			return false;
		}

		const targetDuplicate = report.target_duplicates.find(
			(d) => d.used_by.find((t2) => t2.target.id === target.target.id)!
		)!;

		const isImpossiblePermutation =
			regionalUnionSumTotal(targetDuplicate.teams_with_group_set) === 0;

		if (isImpossiblePermutation) {
			return false;
		}

		const targetMatchCount = report.target_match_count.find(
			(supReqEntry) => supReqEntry.target.target.id === target.target.id
		);

		const notEnoughToPlay =
			(targetMatchCount === undefined ? 0 : regionalUnionSumTotal(targetMatchCount.required)) === 0;

		if (notEnoughToPlay) {
			return false;
		}

		const hasEnoughSupplied =
			targetMatchCount === undefined ? false : isSupplyRequireEntryAccountedFor(targetMatchCount);

		return hasEnoughSupplied;
	}

	let normalSeasonGamesToPlay = 2;
	let normalSeasonInter = false;
	let postSeasonGamesToPlay: number = 1;
	let postSeasonInter = true;

	function reservationTypeGetter(reservationTypeId: number): ReservationType | undefined {
		return reservationTypes?.find((ty) => ty.id === reservationTypeId);
	}

	let inputs_for_scheduling: ScheduledInput[] | undefined;

	let scheduled_output: Promise<ScheduledOutput[]> | undefined;

	let scheduling: boolean = false;

	async function beginScheduleTransaction() {
		try {
			if (!$authStore.isLoggedIn) {
				toastStore.trigger({
					message: 'You must be signed in to send a schedule request',
					background: 'variant-filled-error'
				});

				return;
			}

			if (schedulerWait) {
				toastStore.trigger({
					message:
						'Please slow down! Your account must wait 30 seconds between requesting a schedule.',
					background: 'variant-filled-warning'
				});
				return;
			} else {
				toastStore.trigger({
					message: 'Bundling your payload and sending it to our servers...',
					background: 'variant-filled-tertiary'
				});
			}

			inputs_for_scheduling = await invoke<ScheduledInput[]>('generate_schedule_payload');

			const jwtToken = await getAuth().currentUser!.getIdToken();

			scheduling = true;
			scheduled_output = invoke<ScheduledInput[]>('schedule', { authorizationToken: jwtToken });

			schedulerWait = true;

			setTimeout(() => {
				schedulerWait = false;
			}, SCHEDULE_CREATION_DELAY);

			await scheduled_output;

			toastStore.trigger({
				message: 'The server finished its work!',
				background: 'variant-filled-success'
			});

			scheduling = false;
		} catch (e) {
			console.error(e);

			toastStore.trigger({
				message: `⚠️ Could not schedule: ${JSON.stringify(e)}`,
				autohide: false,
				background: 'variant-filled-error'
			});
		}
	}
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Scheduler</h1>

	<section class="card m-4 p-4">
		<h2 class="h3">Input Parameters</h2>

		<Accordion class="my-4">
			<AccordionItem>
				<svelte:fragment slot="summary">Field Sizes</svelte:fragment>
				<svelte:fragment slot="content">
					{#if reservationTypes === undefined}
						<ProgressRadial />
					{:else if reservationTypes.length === 0}
						<span class="ml-4">You have not created any reservation types.</span>
					{:else}
						<blockquote class="blockquote">
							These are the amount of concurrent games that can be played on each field type.
						</blockquote>
						<hr class="hr" />

						{#if customResTypeSizePerField?.length ?? 0 !== 0}
							<div>
								<span style="color: red">*</span> = custom reservation type/field partitioning in place.
							</div>
						{/if}
						<div class="grid grid-cols-2 gap-8 xl:grid-cols-3">
							{#each reservationTypes as reservationType}
								<div class="block p-5" style="background-color: {reservationType.color}">
									<strong>
										{reservationType.name}
									</strong>
									{#if customResTypeSizePerField === undefined}
										<ProgressRadial />
									{:else}
										<ul class="list" id="field-size-dist">
											{#each customResTypeSizePerField.filter((fc) => fc.reservation_type_id === reservationType.id) as nonDefaultAssociation}
												<li>
													{#await loadField(nonDefaultAssociation.field_id)}
														<ProgressRadial />
													{:then field}
														<span>{field.name}</span>
														<span class="!ml-0" style="color: red">*</span
														>{nonDefaultAssociation.concurrency}
													{:catch error}
														Error: {JSON.stringify(error)}
													{/await}
												</li>
											{/each}

											<li>
												<span>Default</span>
												{reservationType.default_sizing}
											</li>
										</ul>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</svelte:fragment>
			</AccordionItem>
			<AccordionItem>
				<svelte:fragment slot="summary">Time Slots</svelte:fragment>
				<svelte:fragment slot="content">
					<blockquote class="blockquote">
						These are the time slots that you've created across your regions. They will each be
						candidates for scheduling.
					</blockquote>
					<hr class="hr" />

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
					<blockquote class="blockquote">
						These are the teams that you've created across your regions. They will be scheduled
						according to the ruleset you define for a given schedule.
					</blockquote>
					<hr class="hr" />

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

	<section class="card m-4 p-4">
		<h2 class="h3 mb-4">
			Targets
			{#if normalSeasonReport?.target_has_duplicates.length ?? 0 > 0}
				({normalSeasonReport?.target_has_duplicates.length} error{normalSeasonReport
					?.target_has_duplicates.length === 1
					? ''
					: 's'})
			{/if}
		</h2>

		{#if groups === undefined || targets === undefined || reservationTypes === undefined}
			<ProgressRadial />
		{:else}
			{#if targets.length === 0}
				<div class="m-4 text-center" in:send={{ key }} out:receive={{ key }}>
					<p>You haven't created any schedule output targets.</p>
					<p class="mt-4">
						Create a target derived from a combination of the {groups.length}
						group{groups.length > 1 ? 's' : ''} you've created,
					</p>
				</div>
			{:else}
				<div
					class="grid grid-cols-1 gap-8 p-8 lg:grid-cols-2 2xl:grid-cols-3"
					in:send={{ key }}
					out:receive={{ key }}
				>
					{#each targets as target, i}
						{@const normalOK =
							normalSeasonReport !== undefined && !willSendReport
								? isTargetOk(normalSeasonReport, target)
								: false}
						{@const postOK =
							normalSeasonReport !== undefined && !willSendReport && postSeasonReport !== undefined
								? isTargetOk(postSeasonReport, target)
								: false}

						<Target
							id="target-{target.target.id}"
							{groups}
							{target}
							{reservationTypes}
							popupId={i}
							ok={normalOK && postOK}
							on:delete={async (e) => await deleteTarget(e.detail, i)}
							on:groupAdd={async (e) => await targetAddGroup(target, e.detail)}
							on:groupDelete={async (e) => await targetDeleteGroup(target, e.detail)}
							on:modifyReservationType={async (e) => await modifyReservationType(target, e.detail)}
						/>
					{/each}
				</div>
			{/if}

			<hr class="hr my-5" />

			<button
				disabled={groups.length === 0}
				class="variant-filled btn mx-auto block"
				on:click={createTarget}>+ New Target</button
			>

			{#if groups.length === 0}
				<div class="card bg-warning-500 m-4 p-4 text-center">
					You can't create any targets, as you have not created any groups!
					<br />
					<a class="btn underline" href="/groups">Create a group here</a>
				</div>
			{/if}
		{/if}
	</section>

	<hr class="hr" />

	{#if groups?.length ?? 0 !== 0}
		<section class="m-4">
			<h2 class="h2 mb-4">Matches to Play</h2>
			<div class="flex items-center justify-between">
				<Accordion>
					<AccordionItem>
						<svelte:fragment slot="summary">
							<strong> Normal Season </strong>
						</svelte:fragment>
						<svelte:fragment slot="content">
							<RangeSlider
								name="range-slider"
								on:change={updateTargets}
								bind:value={normalSeasonGamesToPlay}
								min={1}
								max={7}
								step={1}
								ticked
							>
								<div class="flex items-center">
									<span>
										Every team

										{#if normalSeasonInter}
											across all regions
										{:else}
											(limited to a region)
										{/if}

										will play each other {normalSeasonGamesToPlay} time{normalSeasonGamesToPlay ===
										1
											? ''
											: 's'}
									</span>

									<div class="grow" />

									<label class="mx-4" for="normal-season-inter">
										Allow interregional matches?

										<span
											class:text-red-500={!normalSeasonInter}
											class:text-green-500={normalSeasonInter}
										>
											{normalSeasonInter ? 'yes' : 'no'}
										</span>
									</label>
									<SlideToggle
										size="sm"
										name="normal-season-inter"
										bind:checked={normalSeasonInter}
										on:change={() => updateTargets()}
									/>
								</div>
							</RangeSlider>
						</svelte:fragment>
					</AccordionItem>
					<AccordionItem>
						<svelte:fragment slot="summary">
							<strong> Post Season </strong>
						</svelte:fragment>
						<svelte:fragment slot="content">
							<RangeSlider
								name="range-slider"
								on:change={() => updateTargets()}
								bind:value={postSeasonGamesToPlay}
								min={1}
								max={7}
								step={1}
								ticked
							>
								<div class="flex items-center">
									<span>
										Every team

										{#if postSeasonInter}
											across all regions
										{:else}
											(limited to a region)
										{/if}

										will play each other {postSeasonGamesToPlay} time{postSeasonGamesToPlay === 1
											? ''
											: 's'}
									</span>

									<div class="grow" />

									<label class="mx-4" for="normal-season-inter">
										Allow interregional matches?

										<span class={postSeasonInter ? 'text-green-500' : 'text-red-500'}>
											{postSeasonInter ? 'yes' : 'no'}
										</span>
									</label>
									<SlideToggle
										size="sm"
										name="normal-season-inter"
										bind:checked={postSeasonInter}
										on:change={() => updateTargets()}
									/>
								</div>
							</RangeSlider>
						</svelte:fragment>
					</AccordionItem>
				</Accordion>
			</div>
		</section>

		<hr class="hr" />

		<section class="m-4">
			<h2 class="h2 mb-4">Reporting</h2>

			{#if willSendReport}
				<section class="grid w-full grid-cols-[auto_1fr] gap-16">
					<div class="p-4">
						<div class="placeholder h-8 md:h-8 md:w-48" />
						<div class="placeholder-circle mx-auto my-4 w-36 animate-pulse" />
					</div>

					<div class="space-y-4 p-4">
						<div class="placeholder" />
						<div class="grid grid-cols-3 gap-8">
							<div class="placeholder" />
							<div class="placeholder" />
							<div class="placeholder" />
						</div>
						<div class="grid grid-cols-4 gap-4">
							<div class="placeholder" />
							<div class="placeholder" />
							<div class="placeholder" />
							<div class="placeholder" />
						</div>
						<div class="placeholder" />
					</div>
				</section>
				<!-- <ProgressBar class="my-auto" /> -->
			{:else}
				<Accordion>
					{#if normalSeasonReport !== undefined}
						<AccordionItem open>
							<svelte:fragment slot="summary">
								<h3 class="h3">Normal Season</h3>
								<ScheduleErrorReport report={normalSeasonReport} />
							</svelte:fragment>
							<svelte:fragment slot="content">
								<ReportTable
									report={normalSeasonReport}
									{reservationTypeGetter}
									regionGetter={loadRegion}
								/>
								<hr class="hr" />
							</svelte:fragment>
						</AccordionItem>
					{:else}
						<ProgressRadial />
					{/if}
					{#if postSeasonReport !== undefined}
						<AccordionItem open>
							<svelte:fragment slot="summary">
								<h3 class="h3">Post Season</h3>
								<ScheduleErrorReport report={postSeasonReport} />
							</svelte:fragment>
							<svelte:fragment slot="content">
								<ReportTable
									report={postSeasonReport}
									previousReport={normalSeasonReport}
									{reservationTypeGetter}
									regionGetter={loadRegion}
								/>
							</svelte:fragment>
						</AccordionItem>
					{/if}
				</Accordion>
			{/if}

			{#if $authStore.isLoggedIn}
				<button
					id="schedule-btn"
					class="variant-filled btn btn-xl mx-auto block"
					on:click={beginScheduleTransaction}
				>
					Schedule
				</button>

				{#if inputs_for_scheduling !== undefined}
					<div class="mt-5">
						<Accordion>
							<AccordionItem disabled={scheduled_output === undefined || scheduling}>
								<svelte:fragment slot="summary">
									<h3 class="h3">
										Output
										{#if scheduling}
											(loading...)
										{/if}
									</h3>
								</svelte:fragment>
								<svelte:fragment slot="content">
									{#if scheduled_output !== undefined}
										{#await scheduled_output}
											<div class="my-5">
												<div class="my-5 text-center">Talking to our server, hold tight...</div>

												<ProgressRadial class="mx-auto block" />
											</div>
										{:then scheduled_output}
											{@const code = JSON.stringify(scheduled_output, null, 4)}
											<CodeBlock language="json" {code} />
										{/await}
									{:else}
										<div class="text-center">
											<strong>No Output!</strong>
										</div>
									{/if}
								</svelte:fragment>
							</AccordionItem>
							<AccordionItem>
								<svelte:fragment slot="summary">
									<h3 class="h3">Input</h3>
								</svelte:fragment>
								<svelte:fragment slot="content">
									{#each inputs_for_scheduling as input_payload}
										{@const code = JSON.stringify(input_payload, null, 4)}
										<div class="mt-4">
											<CodeBlock language="json" {code} />
										</div>
									{:else}
										<div class="text-center">
											<strong>No Payloads!</strong>
										</div>
									{/each}
								</svelte:fragment>
							</AccordionItem>
						</Accordion>
					</div>
				{/if}
			{:else}
				<hr class="hr my-5" />

				<div class="card bg-warning-500 mx-auto w-4/5 p-4 text-center lg:w-1/2">
					<p>
						You must be logged in to send a schedule request to our servers at this time. We do this
						to limit spam and block malicious requests, and hope you understand!
					</p>

					<button
						class="variant-filled btn mt-2"
						on:click={() => goto('/login?next=/scheduler#schedule-btn')}
					>
						Please Sign In
					</button>
				</div>
			{/if}
		</section>
	{:else}
		<section class="card m-4 p-4">
			You will be able to preview your schedule control parameters here once you add a target.
		</section>
	{/if}
</main>

<style>
	#field-size-dist > li > span:first-child {
		display: flex;
		width: 100%;
		margin-right: 0.5em;
	}

	#field-size-dist > li > span:first-child::after {
		content: '';
		flex-grow: 1;
		height: 5px;
		border-bottom: dotted black 2px;
		position: relative;
		bottom: 0;
		transform: translateY(150%);
		margin-left: 0.5em;
	}
</style>
