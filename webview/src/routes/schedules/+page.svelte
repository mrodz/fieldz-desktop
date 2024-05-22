<script lang="ts">
	import type { Schedule } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api';
	import { ProgressRadial } from '@skeletonlabs/skeleton';
	import ScheduleCard from './ScheduleCard.svelte';

	let schedules: Promise<Schedule[]> | undefined;

	onMount(async () => {
		schedules = invoke('get_schedules');
	});

	function cardDeletion(index: number) {
		if (schedules !== undefined) {
			schedules = schedules.then((schedules) => {
				schedules.splice(index, 1);
				return schedules;
			});
		}
	}

	function cardUpdate(event: CustomEvent<{ prev: Schedule; new: Schedule }>) {
		if (schedules !== undefined) {
			schedules = schedules.then((schedules) => {
				for (let i = 0; i < schedules.length; i++) {
					if (schedules[i].id === event.detail.new.id) {
						schedules.splice(i, 1)[0];
						schedules.splice(0, 0, event.detail.new);
						break;
					}
				}
				return schedules;
			});
		}
	}
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Schedules</h1>

	<div class="my-4">
		Here is an overview of the schedules you have already created. Create schedules
		<a class="underline" href="/scheduler">here</a>.
	</div>

	<hr class="hr my-5" />

	<div class="grid grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3 min-[2200px]:grid-cols-4">
		{#if schedules === undefined}
			<ProgressRadial />
		{:else}
			{#await schedules}
				<ProgressRadial />
			{:then schedules}
				{#each schedules as schedule, i}
					<ScheduleCard on:delete={() => cardDeletion(i)} on:update={cardUpdate} {schedule} />
				{:else}
					<div class="block mx-auto mt-4">You haven't generated a schedule yet.</div>
				{/each}
			{/await}
		{/if}
	</div>
</main>
