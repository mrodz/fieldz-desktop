<script lang="ts">
	import { SHOW_SCHEDULER_URL_WHILE_WAITING, type HealthCheck } from '$lib';
	import { getModalStore, ProgressRadial } from '@skeletonlabs/skeleton';
	import { invoke } from '@tauri-apps/api';
	import { onDestroy, onMount } from 'svelte';

	// suppress DOM warnings for not specifying `parent` as a prop
	export let parent: any;
	// use the variable to suppress linter warnings for unused variable
	(() => parent)();

	const modalStore = getModalStore();

	let backendHealth: Promise<HealthCheck> | undefined;
	const start = Date.now();

	let timeString: string;
	let mounted: boolean = false;

	let counter: NodeJS.Timeout | null = setInterval(() => {
		const now = Date.now();

		const minutesDiff = ((now - start) / 60_000) | 0;
		const secondsDiff = ((now - start - minutesDiff * 60_000) / 1_000) | 0;
		const millisDiff = now - start - secondsDiff * 1_000;

		let ss = String(secondsDiff).padStart(2, '0');
		let mmm = String(millisDiff).padStart(3, '0');

		timeString = `${minutesDiff}:${ss}:${mmm}`;
	}, 50);

	onMount(async () => {
		mounted = true;
		try {
			backendHealth = invoke<HealthCheck>('health_probe');
			backendHealth.then((backendHealth) => {
				if (mounted && $modalStore[0] !== undefined) {
					const { onPing } = $modalStore[0].meta;
					if (typeof onPing === 'function') onPing(backendHealth);
				}

				if (backendHealth !== 'Serving' && counter !== null) {
					clearInterval(counter);
					counter = null;
				}
			});
		} catch (e) {
			if (counter !== null) clearInterval(counter);
		}
	});

	onDestroy(() => {
		mounted = false;
	});
</script>

<div class="card w-modal p-5">
	<header class="card-header grid grid-cols-[1fr_auto] items-center">
		<span>
			<h3 class="h3">You submitted a schedule request!</h3>
		</span>

		{#if backendHealth !== undefined}
			<span class="ml-4">
				{#await backendHealth}
					<ProgressRadial width="w-8" />
				{:then backendHealth}
					{@const color = backendHealth === 'Serving' ? 'bg-green-500 animate-ping' : 'bg-red-500'}
					<span class="badge-icon inline-block {color}" />
				{:catch}
					<span class="badge-icon inline-block bg-red-500" />
				{/await}
			</span>
		{/if}
	</header>
	<section class="p-4">
		<strong>What now?</strong>
		<p>
			Please hold tight. Your request to generate a schedule needs to be picked up by a server.
			Sometimes, this means a new server has to wake up to help handle your request.
		</p>
		<p>
			The indicator bubble can let you know the status of your request. When it is spinning, this
			means it is waiting to match with a server. Green means a server was found; red means your
			request could not be completed.
		</p>
		<p>
			Scheduling is an intensive computation. Larger schedules will take more time to complete.
			Please be patient while we crunch the numbers.
		</p>
	</section>
	<section class="text-center">
		Elapsed: {timeString}
	</section>
	{#if SHOW_SCHEDULER_URL_WHILE_WAITING}
		<section class="text-center">
			Target: {#await invoke('get_scheduler_url')}
				Loading...
			{:then url}
				{url}
			{/await}
		</section>
	{/if}
</div>
