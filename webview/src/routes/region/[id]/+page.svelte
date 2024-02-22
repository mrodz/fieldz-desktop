<script lang="ts">
	import type { Region, Field } from '$lib';
	import { onMount } from 'svelte';
	import { slide } from 'svelte/transition';
	import { dialog, invoke } from '@tauri-apps/api';
	import { ProgressRadial, CodeBlock, getModalStore, getToastStore } from '@skeletonlabs/skeleton';

	export let data;

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	let region: Region | undefined;
	let fields: Field[] | undefined;

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

			[region, fields] = await Promise.all([
				invoke<Region>('load_region', { id }),
				invoke<Field[]>('get_fields', { regionId: id })
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
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<button class="variant-filled btn" on:click={() => history.back()}>&laquo;&nbsp;Back</button>

	{#if region === undefined || fields === undefined}
		<div class="placeholder" />
		<ProgressRadial />
	{:else}
		<h1 class="h2 my-4">{region.title}</h1>

		{#if fields.length === 0}
			<div class="m-4 p-4 text-center">⚠️ This region has no fields</div>
			<button class="btn variant-filled block mx-auto" on:click={createField}>Create your first field</button>
		{:else}
			<div class="flex flex-wrap items-stretch justify-center">
				{#each fields as field}
					<div class="card m-4 w-52 lg:w-96 p-4">
						<strong>{field.name}</strong>
						<hr class="my-4" />
						<CodeBlock language="json" lineNumbers code={JSON.stringify(field)} />
					</div>
				{/each}
				<div class="ml-10 my-auto flex flex-col">
					<button class="btn-icon variant-filled mx-auto block h-[75px] w-[75px]" on:click={createField}>+</button>
					<span class="mx-auto mt-2 block">Create Field</span>
				</div>
			</div>
		{/if}
	{/if}
</main>
