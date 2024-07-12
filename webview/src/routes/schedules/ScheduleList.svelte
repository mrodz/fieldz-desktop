<script lang="ts">
	import { ProgressRadial } from '@skeletonlabs/skeleton';
	import ScheduleCard from './ScheduleCard.svelte';
	import type { Schedule } from '$lib';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api';

	export let src: string;
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
						schedules.splice(i, 1);
						schedules.splice(0, 0, event.detail.new);
						break;
					}
				}
				return schedules;
			});
		}
	}
</script>

{#if schedules === undefined}
	<ProgressRadial />
{:else}
	{#await schedules}
		<ProgressRadial />
	{:then schedules}
		{#if schedules.length !== 0}
			<div
				class="relative grid grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3 min-[2200px]:grid-cols-4"
			>
				{#each schedules as schedule, i}
					<ScheduleCard {src} on:delete={() => cardDeletion(i)} on:update={cardUpdate} {schedule} />
				{/each}
			</div>
			<div class="my-auto ml-10 flex flex-col">
				<a href="/scheduler" class="variant-filled btn-icon mx-auto block flex h-[75px] w-[75px]">
					+
				</a>
				<span class="mx-auto mt-2 block">Create Schedule</span>
			</div>
		{:else}
			<div class="mx-auto text-center">
				You haven't generated a schedule yet.
				<div class="my-auto mt-6 flex flex-col">
					<a href="/scheduler" class="variant-filled btn-icon mx-auto block flex h-[75px] w-[75px]">
						+
					</a>
					<span class="mx-auto mt-2 block">Create Schedule</span>
				</div>
			</div>
		{/if}
	{/await}
{/if}
