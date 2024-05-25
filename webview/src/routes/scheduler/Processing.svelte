<script lang="ts">
	import { type HealthCheck } from '$lib';
	import { ProgressRadial } from '@skeletonlabs/skeleton';
	import { invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	let backendHealth: Promise<HealthCheck> | undefined;
	const start = Date.now();

	let timeString: string;

	const counter = setInterval(() => {
		const now = Date.now();

		const minutesDiff = ((now - start) / 60_000) | 0;
		const secondsDiff = ((now - start - minutesDiff * 60_000) / 1_000) | 0;
		const millisDiff = now - start - secondsDiff * 1_000;

		let ss = String(secondsDiff).padStart(2, '0');
		let mmm = String(millisDiff).padStart(3, '0');

		timeString = `${minutesDiff}:${ss}:${mmm}`;
	}, 50);

	onMount(async () => {
		try {
			backendHealth = invoke<HealthCheck>('health_probe');
			if ((await backendHealth) !== 'Serving') clearInterval(counter);
		} catch (e) {
			clearInterval(counter);
		}
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
</div>
