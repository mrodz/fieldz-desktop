<script lang="ts">
	import type { Region, Field, Team, TeamExtension } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { dialog, invoke } from '@tauri-apps/api';
	import { ProgressRadial, getModalStore, getToastStore } from '@skeletonlabs/skeleton';

	export let data;

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let region: Region | undefined;
	let fields: Field[] | undefined;
	let teams: TeamExtension[] | undefined;

	onMount(async () => {
		try {
			const id = Number(data.id);

			if (isNaN(id)) {
				dialog.message(`[id = ${data.id}] is NaN`, {
					title: 'Error',
					type: 'error'
				});
				return;
			}

			[region, fields, teams] = await Promise.all([
				invoke<Region>('load_region', { id }),
				invoke<Field[]>('get_fields', { regionId: id }),
				invoke<TeamExtension[]>('get_teams_and_tags', { regionId: id })
			]);
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Error',
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

		if (typeof data.id === 'string' && data.id.length > 0) {
			id = Number(data.id);
		} else {
			toastStore.trigger({
				message: `Could not edit region with non-str id: ${data.id}`,
				background: 'variant-filled-success'
			});
			return;
		}

		if (isNaN(id)) {
			toastStore.trigger({
				message: `Could not edit region with non-int id: ${data.id}`,
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
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp;Regions</button>

	{#if region === undefined || fields === undefined || teams === undefined}
		<div class="placeholder" />
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4 flex">
			{region.title}
			<button class="variant-ghost btn-icon my-auto ml-4" on:click={editRegion}>
				<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 -960 960 960" width="24">
					<path
						d="M200-200h57l391-391-57-57-391 391v57Zm-80 80v-170l528-527q12-11 26.5-17t30.5-6q16 0 31 6t26 18l55 56q12 11 17.5 26t5.5 30q0 16-5.5 30.5T817-647L290-120H120Zm640-584-56-56 56 56Zm-141 85-28-29 57 57-29-28Z"
					/>
				</svg>
			</button>
		</h1>

		<hr class="my-4" />

		<div class="grid grid-cols-1 grid-rows-2 lg:grid-cols-2 lg:grid-rows-1">
			<section class="card m-4 p-4">
				<h2 class="h2 text-center">Teams</h2>
				{#if teams.length === 0}
					<div class="m-4 p-4 text-center">⚠️ This region has no teams</div>
					<button class="variant-filled btn mx-auto block" on:click={createTeam}
						>Create your first team</button
					>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each teams as team_ext, i}
							<div class="card m-4 w-52 p-4 lg:w-96">
								<header class="card-header flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{team_ext.team.name}</strong>
									<button
										type="button"
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteTeam(team_ext.team, i)}>X</button
									>
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

								<hr class="my-4" />
								<button
									class="variant-filled btn mx-auto block"
									on:click={() => editTeam(team_ext, i)}>Edit</button
								>
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
				<h2 class="h2 text-center">Fields</h2>

				{#if fields.length === 0}
					<div class="m-4 p-4 text-center">⚠️ This region has no fields</div>
					<button class="variant-filled btn mx-auto block" on:click={createField}
						>Create your first field</button
					>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each fields as field, i}
							<div class="card m-4 w-52 p-4 lg:w-96">
								<header class="card-header mb-4 flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{field.name}</strong>
									<button
										type="button"
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteField(field, i)}>X</button
									>
								</header>

								<a class="variant-filled btn mx-auto block" href={`/reservations/${field.id}`}>
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
