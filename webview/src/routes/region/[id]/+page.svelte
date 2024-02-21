<script lang="ts">
	import type { Region } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { dialog, invoke } from '@tauri-apps/api';
	import { ProgressRadial, CodeBlock } from '@skeletonlabs/skeleton';

	export let data;

	let region: Region | undefined;

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

			region = await invoke('load_region', { id });
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Error',
				type: 'error'
			});
		}
	});
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp;Back</button>
	<h1 class="h2 my-4">Viewing Region</h1>

	{#if region === undefined}
		<ProgressRadial />
	{:else}
		<CodeBlock language="json" code={JSON.stringify(region)}></CodeBlock>
	{/if}
</main>
