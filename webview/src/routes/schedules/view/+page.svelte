<script lang="ts">
	import type { Schedule } from '$lib';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { ProgressRadial } from '@skeletonlabs/skeleton';

	const queryParams = new URLSearchParams(window.location.search);
	const idParam = queryParams.get('id');

	if (idParam === null || idParam === '') {
		dialog.message(`Recieved a bad query parameter for 'id' (got: ${JSON.stringify(idParam)})`);
		history.back();
	}

	const id: number = Number(idParam);

	if (!Number.isInteger(id)) {
		dialog.message(
			`Recieved a bad query parameter for 'id' (got non-int: ${JSON.stringify(idParam)})`
		);
		history.back();
	}

	let schedule: Promise<Schedule> | undefined;

	onMount(async () => {
		schedule = invoke<Schedule>('get_schedule', { id });
	});
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
			<h1 class="h2">{schedule.name}</h1>
			<button class="variant-filled btn my-4" on:click={() => history.back()}>
				&laquo;&nbsp; Back
			</button>
			<div>
				{JSON.stringify(schedule)}
			</div>
		{/await}
	{/if}
</main>
