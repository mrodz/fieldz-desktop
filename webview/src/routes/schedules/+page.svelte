<script lang="ts">
	import type { Schedule } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { invoke } from '@tauri-apps/api';
	import { ProgressRadial } from '@skeletonlabs/skeleton';
	import ScheduleCard from './ScheduleCard.svelte';

	let schedules: Promise<Schedule[]> | undefined;


	onMount(async () => {
		schedules = invoke('get_schedules');
	});
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Schedules</h1>

	<div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
		{#if schedules === undefined}
			<ProgressRadial />
		{:else}
			{#await schedules}
				<ProgressRadial />
			{:then schedules}
				{#each schedules as schedule}
					<ScheduleCard {schedule} />
				{:else}
					No Schedules!
				{/each}
			{/await}
		{/if}
	</div>
</main>
