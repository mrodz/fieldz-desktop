<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Region } from '$lib';
	import { ProgressRadial, getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	let toastStore = getToastStore();
	let modalStore = getModalStore();
	let regions: Region[] | undefined = undefined;

	export const addRegionToFrontend = (region: Region) => {
		if (regions === undefined) {
			regions = [region];
		} else {
			regions.push(region);
			// force an update
			regions = regions;
		}
	};

	export const removeRegionFromFrontend = (index: number) => {
		if (regions !== undefined) {
			regions.splice(index, 1);
			// force an update
			regions = regions;
		}
	};

	onMount(async () => {
		try {
			const data = await invoke<Region[]>('get_regions');
			if (regions === undefined) {
				regions = data;
			} else {
				regions = regions.concat(data);
			}
		} catch (error) {
			dialog.message('Could not load regions', {
				title: 'Fieldz',
				type: 'error'
			});
		}
	});

	async function deleteRegion(region: Region, index: number) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm',
			body: `Deleting a region is PERMANENT! Are you sure you wish to proceed? You will NOT be able to recover "${region.title}"`,
			buttonTextConfirm: 'Delete',
			async response(r) {
				if (r) {
					try {
						await invoke('delete_region', {
							id: region.id
						});

						toastStore.trigger({
							message: `Deleted "${region.title}"`,
							background: 'variant-filled-success'
						});

						removeRegionFromFrontend(index);
					} catch (e) {
						dialog.message(`Could not delete \`${region.title}\`: ${JSON.stringify(e)}`, {
							title: `Deleting region ${region.id}`,
							type: 'error'
						});
					}
				}
			}
		});
	}
</script>

{#if regions === undefined}
	<ProgressRadial />
{:else if regions.length === 0}
	<div class="m-4 p-4 text-center">You aren't a part of any regions!</div>
{:else}
	<div class="flex flex-wrap justify-center">
		{#each regions as region, i}
			<button
				class="card btn card-hover m-5 block w-96 p-5"
				tabindex="0"
				on:click|preventDefault={() => goto(`/region?id=${region.id}`)}
			>
				<header class="card-header flex flex-row items-center">
					<strong class="w-1/2 grow truncate">{region.title}</strong>
					<button
						type="button"
						class="variant-filled btn-icon"
						on:click|stopPropagation={() => deleteRegion(region, i)}>X</button
					>
				</header>
			</button>
		{/each}
	</div>
{/if}
