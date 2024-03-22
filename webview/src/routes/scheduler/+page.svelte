<script lang="ts">
	import { blur, slide, crossfade } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
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
		eventFromTimeSlot,
		type TeamGroup,
		type TargetExtension,
		type PreScheduleReport,
		type PreScheduleReportInput
	} from '$lib';
	import {
		getModalStore,
		Accordion,
		AccordionItem,
		ProgressBar,
		Paginator,
		SlideToggle,
		Table,
		type PaginationSettings,
		type TableSource,
		ProgressRadial,
		RangeSlider
	} from '@skeletonlabs/skeleton';

	import { dialog, event, invoke } from '@tauri-apps/api';
	import Target from './Target.svelte';

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

	let teams: TeamExtension[] | undefined;
	let groups: TeamGroup[] | undefined;
	let targets: TargetExtension[] | undefined;

	onMount(async () => {
		try {
			teams = await invoke<TeamExtension[]>('load_all_teams');
			groups = await invoke<TeamGroup[]>('get_groups');
			targets = await invoke<TargetExtension[]>('get_targets');
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
	let report: PreScheduleReport | undefined;
	let willSendReport = false;

	async function generateReport() {
		try {
			const input = {
				matches_to_play: gamesToPlay
			} satisfies PreScheduleReportInput;

			report = await invoke<PreScheduleReport>('generate_pre_schedule_report', { input });
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
				title: `Error adding group to target`,
				type: 'error'
			});
		}
	}

	const [send, receive] = crossfade({
		duration: 250,
		easing: quintOut
	});

	const key = Symbol('key for crossfade animation');

	function percentFillage(totalMatchesRequired: number, totalMatchesSupplied: number): number {
		if (totalMatchesRequired === 0) {
			totalMatchesSupplied = 1;
		} else {
			totalMatchesSupplied /= totalMatchesRequired;
		}

		return Math.round(totalMatchesSupplied * 100);
	}

	$: reportTableSource = {
		head: ['ID', 'Groups', '# of Teams', 'Matches Required'],
		body:
			report?.target_required_matches.map(([target, matches]) => [
				`${target.target.id}${target.groups.length === 0 ? ' ⚠️' : ''}`,
				target.groups.length > 0
					? target.groups
							.map((g) => `<span class="chip variant-filled-success">${g.name}</span>`)
							.join(' ')
					: '<strong>Will Not Schedule</strong>',
				String(
					report?.target_duplicates.find((d) =>
						d.used_by.map((u) => u.target.id).includes(target.target.id)
					)!.teams_with_group_set ?? 0
				) + (matches === 0 ? ' (<i>not enough teams</i>)' : ''),
				String(matches)
			]) ?? []
	} satisfies TableSource;

	function reportHasErrors(report: PreScheduleReport): boolean {
		return (
			report.target_has_duplicates.length !== 0 ||
			report.target_duplicates.find((d) => d.teams_with_group_set === 0) !== undefined ||
			report.target_required_matches.find(([_, occ]) => occ === 0) !== undefined
		);
	}

	function isTargetOk(report: PreScheduleReport, target: TargetExtension): boolean {
		const isDuplicate = report.target_has_duplicates.includes(target.target.id);

		if (isDuplicate) {
			return false;
		}

		const isImpossiblePermutation =
			report.target_duplicates.find(
				(d) => d.used_by.find((t2) => t2.target.id === target.target.id)!
			)!.teams_with_group_set === 0;

		if (isImpossiblePermutation) {
			return false;
		}

		const notEnoughToPlay =
			report.target_required_matches.find(([t2]) => t2.target.id === target.target.id)![1] === 0;

		if (notEnoughToPlay) {
			return false;
		}

		return true;
	}

	let gamesToPlay = 2;
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

	<section class="card m-4 p-4">
		<h2 class="h3 mb-4">
			Targets
			{#if report?.target_has_duplicates.length ?? 0 > 0}
				({report?.target_has_duplicates.length} error{report?.target_has_duplicates.length === 1
					? ''
					: 's'})
			{/if}
		</h2>

		{#if groups === undefined || targets === undefined}
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
						<Target
							id="target-{target.target.id}"
							{groups}
							{target}
							popupId={i}
							ok={report !== undefined ? isTargetOk(report, target) : false}
							on:delete={async (e) => await deleteTarget(e.detail, i)}
							on:groupAdd={async (e) => await targetAddGroup(target, e.detail)}
							on:groupDelete={async (e) => await targetDeleteGroup(target, e.detail)}
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

	<section class="m-4">
		<h2 class="h2 mb-4">Matches to Play</h2>
		<RangeSlider
			name="range-slider"
			on:change={() => updateTargets()}
			bind:value={gamesToPlay}
			min={1}
			max={7}
			step={1}
			ticked
		>
			<div class="flex items-center justify-between">
				<div>
					Every team will play each other {gamesToPlay} time{gamesToPlay === 1 ? '' : 's'}
				</div>
			</div>
		</RangeSlider>
	</section>

	<section class="m-4">
		<h2 class="h2 mb-4">Reporting</h2>

		{#if willSendReport}
			<section class="card w-full grid grid-cols-[auto_1fr] gap-16">
				<div class="placeholder-circle mt-4 ml-4 w-32 animate-pulse" />

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
		{:else if report !== undefined}
			{#if reportHasErrors(report)}
				<div class="card bg-error-400 m-4 grid gap-4 p-4 text-center">
					{#if report.target_has_duplicates.length !== 0}
						<div>
							<strong>Cannot use targets because of duplicates</strong>
							<ul class="list">
								{#each report.target_duplicates.filter((d) => d.used_by.length > 1) as dup}
									<li>
										<span>Duplicates on targets</span>

										{#each dup.used_by as badTarget}
											<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
												>{badTarget.target.id}</a
											>
										{/each}

										<span>which use labels</span>

										{#each dup.team_groups as group}
											<span class="variant-filled chip">{group.name}</span>
										{/each}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if report.target_duplicates.find((d) => d.teams_with_group_set === 0) !== undefined}
						<div>
							<strong>Cannot use targets because no team has the following sets of labels</strong>
							<ul class="list">
								{#each report.target_duplicates.filter((d) => d.teams_with_group_set === 0) as empty}
									<li>
										<span>Target(s)</span>

										{#each empty.used_by as badTarget}
											<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
												>{badTarget.target.id}</a
											>
										{/each}

										{#if empty.team_groups.length === 0}
											<span>reference(s) impossible team which uses no labels</span>
										{:else}
											<span>reference(s) impossible team which uses labels</span>

											{#each empty.team_groups as group}
												<span class="variant-filled chip">{group.name}</span>
											{/each}
										{/if}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if report.target_required_matches.find(([_, occ]) => occ === 0) !== undefined}
						<div>
							<strong>Cannot use targets because no games will be outputted</strong>
							<ul class="list">
								{#each report.target_required_matches.filter(([_, occ]) => occ === 0) as [badTarget]}
									<li>
										<span>Target</span>

										<a class="variant-filled-error chip" href="#target-{badTarget.target.id}"
											>{badTarget.target.id}</a
										>

										{#if badTarget.groups.length === 0}
											<span>is empty and will not create any games</span>
										{:else}
											<span>which use labels</span>

											{#each badTarget.groups as group}
												<span class="variant-filled chip">{group.name}</span>
											{/each}

											<span>will not create any games</span>
										{/if}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			{/if}

			<div class="grid grid-cols-[auto_1fr] gap-16">
				<div>
					<h3 class="h4">Matches Supplied / Required</h3>
					<ProgressRadial
						class="mx-auto my-4"
						strokeLinecap="round"
						meter={report.total_matches_required <= report.total_matches_supplied
							? 'stroke-success-500'
							: 'stroke-warning-500'}
						track={report.total_matches_required <= report.total_matches_supplied
							? 'stroke-success-500/30'
							: 'stroke-warning-500/30'}
						value={percentFillage(report.total_matches_required, report.total_matches_supplied)}
					>
						{report.total_matches_supplied}/{report.total_matches_required}
					</ProgressRadial>
				</div>

				<div>
					<h3 class="h4">Per target</h3>

					<Table class="my-4" source={reportTableSource} />
				</div>
			</div>
		{:else}
			test {report}
		{/if}
	</section>
</main>
