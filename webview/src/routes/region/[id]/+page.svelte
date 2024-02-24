<script lang="ts">
	import type { Region, Field, Team, TeamGroup } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { dialog, invoke } from '@tauri-apps/api';
	import { ProgressRadial, CodeBlock, getModalStore, getToastStore } from '@skeletonlabs/skeleton';

	export let data;

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let region: Region | undefined;
	let fields: Field[] | undefined;
	let teams: Team[] | undefined;

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
				invoke<Team[]>('get_teams', { regionId: id }),
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
				onCreate(team: Team) {
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
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp;Regions</button>

	{#if region === undefined || fields === undefined || teams === undefined}
		<div class="placeholder" />
		<ProgressRadial />
	{:else}
		<h1 class="h1 my-4">{region.title}</h1>

		<hr class="my-4" />

		<div class="grid grid-rows-2 grid-cols-1 lg:grid-rows-1 lg:grid-cols-2">
			<section class="card m-4 p-4">
				<h2 class="h2 text-center">Teams</h2>
				{#if teams.length === 0}
					<div class="m-4 p-4 text-center">⚠️ This region has no teams</div>
					<button class="btn variant-filled mx-auto block" on:click={createTeam}
						>Create your first team</button
					>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each teams as team, i}
							<div class="card m-4 w-52 p-4 lg:w-96">
								<header class="card-header flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{team.name}</strong>
									<button
										type="button"
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteTeam(team, i)}>X</button
									>
								</header>

								<hr class="my-4" />

								<CodeBlock language="json" lineNumbers code={JSON.stringify(team)} />

								<hr class="my-4" />
								<button class="btn variant-filled mx-auto block" disabled>Edit</button>
							</div>
						{/each}
						<div class="my-auto ml-10 flex flex-col">
							<button
								class="btn-icon variant-filled mx-auto block h-[75px] w-[75px]"
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
					<button class="btn variant-filled mx-auto block" on:click={createField}
						>Create your first field</button
					>
				{:else}
					<div class="flex flex-wrap items-stretch justify-center">
						{#each fields as field, i}
							<div class="card m-4 w-52 p-4 lg:w-96">
								<header class="card-header flex flex-row items-center">
									<strong class="w-1/2 grow truncate">{field.name}</strong>
									<button
										type="button"
										class="variant-filled btn-icon"
										on:click|stopPropagation={() => deleteField(field, i)}>X</button
									>
								</header>

								<hr class="my-4" />

								<CodeBlock language="json" lineNumbers code={JSON.stringify(field)} />

								<hr class="my-4" />

								<button class="btn variant-filled mx-auto block">Time Slots</button>
							</div>
						{/each}
						<div class="my-auto ml-10 flex flex-col">
							<button
								class="btn-icon variant-filled mx-auto block h-[75px] w-[75px]"
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
