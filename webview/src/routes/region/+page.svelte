<script context="module">
	import { writable } from 'svelte/store';
	let compactTeams = writable(false);
	let compactFields = writable(false);
</script>

<script lang="ts">
	import type { Region, Field, Team, TeamExtension, CoachingConflict } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { dialog, invoke } from '@tauri-apps/api';
	import {
		ProgressRadial,
		getModalStore,
		getToastStore,
		SlideToggle
	} from '@skeletonlabs/skeleton';
	import Fa from 'svelte-fa';
	import { faEdit, faTrash } from '@fortawesome/free-solid-svg-icons';
	import ConflictCard from './ConflictCard.svelte';

	const queryParams = new URLSearchParams(window.location.search);
	const regionId = queryParams.get('id');

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let region: Region | undefined;
	let fields: Field[] | undefined;
	let teams: TeamExtension[] | undefined;
	let conflicts: CoachingConflict[] | undefined;

	const mockConflict = () => {
		if (teams === undefined) return;

		const teamOne = teams[Math.floor(Math.random() * teams.length)];
		let teamTwo;
		do {
			teamTwo = teams[Math.floor(Math.random() * teams.length)];
		} while (teamTwo.team.id === teamOne.team.id);

		return {
			teams: [teamOne.team.id, teamTwo.team.id]
		};
	};

	onMount(async () => {
		try {
			const id = Number(regionId);

			if (isNaN(id)) {
				dialog.message(`[id = ${regionId}] is NaN`, {
					title: 'Could not load region, missing precondition',
					type: 'error'
				});
				return;
			}

			[region, fields, teams, conflicts] = await Promise.all([
				invoke<Region>('load_region', { id }),
				invoke<Field[]>('get_fields', { regionId: id }),
				invoke<TeamExtension[]>('get_teams_and_tags', { regionId: id }),
				[] /* TODO */
			]);

			conflicts = [mockConflict()!];
		} catch (e) {
			console.error(e);
			dialog.message(JSON.stringify(e), {
				title: 'Could not load resources',
				type: 'error'
			});
		}
	});

	async function createField() {
		modalStore.trigger({
			type: 'component',
			component: 'fieldCreate',
			meta: {
				onCreate(field: Field) {
					fields?.push(field);
					fields = fields;
				},
				region
			}
		});
	}

	async function deleteField(field: Field, index: number) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm',
			body: `Deleting a field is PERMANENT! Are you sure you wish to proceed? You will NOT be able to recover "${field.name}"`,
			buttonTextConfirm: 'Delete',
			async response(r) {
				if (r) {
					try {
						await invoke('delete_field', {
							id: field.id
						});

						toastStore.trigger({
							message: `Deleted "${field.name}"`,
							background: 'variant-filled-success'
						});

						fields?.splice(index, 1);
						fields = fields;
					} catch (e) {
						dialog.message(`Could not delete \`${field.name}\`: ${JSON.stringify(e)}`, {
							title: `Deleting field ${field.id}`,
							type: 'error'
						});
					}
				}
			}
		});
	}

	async function createTeam() {
		modalStore.trigger({
			type: 'component',
			component: 'teamCreate',
			meta: {
				onCreate(team: TeamExtension) {
					teams?.push(team);
					teams = teams;
				},
				region
			}
		});
	}

	async function deleteTeam(team: Team, index: number) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm',
			body: `Deleting a team is PERMANENT! Are you sure you wish to proceed? You will NOT be able to recover "${team.name}"`,
			buttonTextConfirm: 'Delete',
			async response(r) {
				if (r) {
					try {
						await invoke('delete_team', {
							id: team.id
						});

						toastStore.trigger({
							message: `Deleted "${team.name}"`,
							background: 'variant-filled-success'
						});

						teams?.splice(index, 1);
						teams = teams;
					} catch (e) {
						dialog.message(`Could not delete \`${team.name}\`: ${JSON.stringify(e)}`, {
							title: `Deleting field ${team.id}`,
							type: 'error'
						});
					}
				}
			}
		});
	}

	let regionNameInput: string | undefined;

	$: if (region !== undefined) {
		regionNameInput = region.title;
	}

	async function editRegion() {
		let id: number;

		if (typeof regionId === 'string' && regionId.length > 0) {
			id = Number(regionId);
		} else {
			toastStore.trigger({
				message: `Could not edit region with non-str id: ${regionId}`,
				background: 'variant-filled-success'
			});
			return;
		}

		if (isNaN(id)) {
			toastStore.trigger({
				message: `Could not edit region with non-int id: ${regionId}`,
				background: 'variant-filled-success'
			});
			return;
		}

		modalStore.trigger({
			type: 'component',
			component: 'regionEdit',
			meta: {
				id,
				onUpdate(updatedRegion: Region) {
					toastStore.trigger({
						message: `Saved changes for "${updatedRegion.title}"`,
						background: 'variant-filled-success'
					});

					region = updatedRegion;
				}
			}
		});
	}

	async function editTeam(team: TeamExtension, index: number) {
		modalStore.trigger({
			type: 'component',
			component: 'teamEdit',
			meta: {
				team,
				onUpdate(updatedTeam: TeamExtension) {
					toastStore.trigger({
						message: `Saved changes for "${updatedTeam.team.name}"`,
						background: 'variant-filled-success'
					});

					teams![index] = updatedTeam;
					teams = teams;
				}
			}
		});
	}

	function teamById(id: number): TeamExtension {
		const result = teams?.find((team_ext) => team_ext.team.id === id);
		if (result === undefined) throw new Error(`team with id = ${id} not found in this region`);
		return result;
	}

	async function deleteConflict(conflict: CoachingConflict, index: number) {
		try {
			/*
			await invoke
			 */

			console.log('DELETING', index, conflicts);
			conflicts?.splice(index, 1);
			conflicts = conflicts;
		} catch (e) {
			console.error(e);
			dialog.message(JSON.stringify(e), {
				title: 'Could not delete coach conflict mapping',
				type: 'error'
			});
		}
	}

	async function updateConflict(
		conflict: CoachingConflict,
		options: {
			nameOnLoad: string | undefined;
			teamsOnLoad: number[];
		}
	) {
		const teamsOnLoadSorted = options.teamsOnLoad.sort();
		if (
			conflict.coach_name === options.nameOnLoad &&
			conflict.teams.sort().every((val, index) => val === teamsOnLoadSorted[index])
		) {
			return; // no changes
		}

		// TODO

		conflicts = conflicts;
	}

	async function createCoachConflictMapping() {
		modalStore.trigger({
			type: 'prompt',
			title: "Enter this coach's name (Optional)",
			response(r: false | undefined | string) {
				if (r === false) {
					return; // the user clicked "cancel"
				}

				// TODO

				conflicts = conflicts;
			}
		});
	}

	async function addTeamToConflict(options: { addTeamToConflict: (team: TeamExtension) => void }) {
		// TODO

		let team: TeamExtension = void 0 as any;

		options.addTeamToConflict(team);
	}
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp;Regions</button>

	{#if region === undefined || fields === undefined || teams === undefined || conflicts === undefined}
		<div class="placeholder" />
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4 flex">
			{region.title}
			<button class="variant-ghost btn-icon my-auto ml-4" on:click={editRegion}>
				<Fa icon={faEdit} />
			</button>
		</h1>

		<hr class="my-4" />

		<div>
			<section class="card m-4 p-4">
				<h2 class="h2">Coaching conflict resolver</h2>
				{#if conflicts.length === 0}
					<i class="my-4 inline-block w-full text-center">
						You haven't specified any coaches who coach multiple teams
					</i>

					<div class="flex flex-col">
						<button
							class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
							on:click={createCoachConflictMapping}>+</button
						>
						<span class="mx-auto mt-2 block">Create coach conflict mapping</span>
					</div>
				{:else}
					<div class="flex flex-col gap-2 md:grid md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
						{#each conflicts as conflict, i}
							<ConflictCard
								on:debouncedUpdate={(e) => updateConflict(e.detail.conflict, e.detail.options)}
								on:delete={(e) => deleteConflict(e.detail, i)}
								on:addTeam={(e) => addTeamToConflict(e.detail)}
								{conflict}
								{teamById}
							/>
						{/each}
						<div class="my-auto flex flex-col md:ml-10">
							<button
								class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
								on:click={createCoachConflictMapping}>+</button
							>
							<span class="mx-auto mt-2 block">Add Coach Rule</span>
						</div>
					</div>
				{/if}
			</section>
		</div>

		<div class="flex flex-col lg:grid lg:grid-cols-2 lg:grid-rows-1">
			<section class="card m-4 p-4">
				<h2 class="h2 text-center">Teams ({teams.length})</h2>

				<SlideToggle name="teams-slider-view" bind:checked={$compactTeams}>
					{#if $compactTeams}
						Expand
					{:else}
						Compact
					{/if}
					Teams
				</SlideToggle>

				<hr class="hr my-5" />

				{#if teams.length === 0}
					<div class="m-4 p-4 text-center">⚠️ This region has no teams</div>
					<button class="variant-filled btn mx-auto block" on:click={createTeam}>
						Create your first team
					</button>
				{:else if $compactTeams}
					<table class="table">
						<thead class="table-head">
							<tr>
								<th role="columnheader">Name</th>
								<th role="columnheader">Tags</th>
								<th role="columnheader">Actions</th>
							</tr>
						</thead>
						<tbody class="table-body">
							{#each teams as team_ext, i}
								<tr aria-rowindex={i + 1}>
									<td role="gridcell" width="1%" aria-colindex="1" tabindex="-1">
										{team_ext.team.name}
									</td>
									<td role="gridcell" width="40%" aria-colindex="2" tabindex="-1">
										{#if team_ext.tags.length !== 0}
											<div>
												{#each team_ext.tags as tag}
													<span class="variant-filled-success chip">{tag.name}</span>
												{/each}
											</div>
										{:else}
											<i>No groups yet!</i>
										{/if}
									</td>
									<td role="gridcell" width="20%" aria-colindex="3" tabindex="-1">
										<div class="grid grid-cols-2 [&>button]:mx-auto lg:[&>button]:mx-0">
											<button
												class="variant-filled btn-icon"
												on:click={() => editTeam(team_ext, i)}
											>
												<Fa icon={faEdit} />
												<span class="sr-only">Edit Team</span>
											</button>
											<button
												class="variant-filled btn-icon"
												on:click|stopPropagation={() => deleteTeam(team_ext.team, i)}
											>
												<Fa icon={faTrash} />
												<span class="sr-only">Delete Team</span>
											</button>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
					<div class="mt-10 flex flex-col">
						<button
							class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
							on:click={createTeam}>+</button
						>
						<span class="mx-auto mt-2 block">Create Team</span>
					</div>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each teams as team_ext, i}
							<div class="card m-4 w-64 p-4 lg:w-96">
								<header class="card-header flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{team_ext.team.name}</strong>
									<button class="variant-filled btn-icon" on:click={() => editTeam(team_ext, i)}>
										<Fa icon={faEdit} />
										<span class="sr-only">Edit Team</span>
									</button>
									<button
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteTeam(team_ext.team, i)}
									>
										<Fa icon={faTrash} />
										<span class="sr-only">Delete Team</span>
									</button>
								</header>

								<hr class="my-4" />

								{#if team_ext.tags.length !== 0}
									<div>
										{#each team_ext.tags as tag}
											<span class="variant-filled-success chip">{tag.name}</span>
										{/each}
									</div>
								{:else}
									<i>No groups yet!</i>
								{/if}
							</div>
						{/each}
						<div class="my-auto ml-10 flex flex-col">
							<button
								class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
								on:click={createTeam}>+</button
							>
							<span class="mx-auto mt-2 block">Create Team</span>
						</div>
					</div>
				{/if}
			</section>
			<section class="card m-4 p-4">
				<h2 class="h2 text-center">Fields ({fields.length})</h2>

				<SlideToggle name="teams-slider-view" bind:checked={$compactFields}>
					{#if $compactFields}
						Expand
					{:else}
						Compact
					{/if}
					Fields
				</SlideToggle>

				<hr class="hr my-5" />

				{#if fields.length === 0}
					<div class="m-4 p-4 text-center">⚠️ This region has no fields</div>
					<button class="variant-filled btn mx-auto block" on:click={createField}>
						Create your first field
					</button>
				{:else if $compactFields}
					<table class="table">
						<thead class="table-head">
							<tr>
								<th class="" role="columnheader">Name</th>
								<th class="" role="columnheader">Actions</th>
							</tr>
						</thead>
						<tbody class="table-body">
							{#each fields as field, i}
								<tr aria-rowindex={i + 1}>
									<td role="gridcell" aria-colindex="1" tabindex="-1">
										{field.name}
									</td>
									<td role="gridcell" width="1%" aria-colindex="3" tabindex="-1">
										<a
											class="variant-filled btn mx-auto block"
											href={`/reservations?fieldId=${field.id}`}
										>
											Edit
										</a>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
					<div class="mt-10 flex flex-col">
						<button
							class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
							on:click={createField}>+</button
						>
						<span class="mx-auto mt-2 block">Create Field</span>
					</div>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each fields as field, i}
							<div class="card m-4 w-52 p-4 lg:w-96">
								<header class="card-header mb-4 flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{field.name}</strong>
									<button
										type="button"
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteField(field, i)}
									>
										<Fa icon={faTrash} />
									</button>
								</header>

								<a
									class="variant-filled btn mx-auto block"
									href={`/reservations?fieldId=${field.id}`}
								>
									Edit Time Slots
								</a>
							</div>
						{/each}
						<div class="my-auto ml-10 flex flex-col">
							<button
								class="variant-filled btn-icon mx-auto block h-[75px] w-[75px]"
								on:click={createField}>+</button
							>
							<span class="mx-auto mt-2 block">Create Field</span>
						</div>
					</div>
				{/if}
			</section>
		</div>
	{/if}
</main>
