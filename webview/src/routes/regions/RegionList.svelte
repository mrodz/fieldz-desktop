<script lang="ts">
	import { goto } from '$app/navigation';
	import type { Region, RegionMetadata } from '$lib';
	import { ProgressRadial, getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';

	let toastStore = getToastStore();
	let modalStore = getModalStore();
	let regions: [Region, Promise<RegionMetadata>][] | undefined = undefined;

	export const addRegionToFrontend = (region: [Region, Promise<RegionMetadata>]) => {
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
			regions = data.map((region) => {
				const metadata = invoke<RegionMetadata>('get_region_metadata', {
					regionId: region.id
				}).catch((e) => {
					toastStore.trigger({
						message: `Could not load region metadata: ${e}`,
						background: 'variant-filled-error',
						timeout: 10_000
					});
					return {
						region_id: region.id,
						field_count: 0,
						team_count: 0,
						time_slot_count: 0
					} satisfies RegionMetadata;
				});
				return [region, metadata];
			});
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
	<div class="m-4 p-4 text-center">You haven't created any regions!</div>
{:else}
	<div class="flex flex-wrap justify-center">
		{#each regions as [region, metadata], i}
			<button
				class="card btn card-hover m-5 block flex w-96 flex-col p-5"
				tabindex="0"
				on:click|preventDefault={() => goto(`/region?id=${region.id}`)}
			>
				<div class="flex w-full flex-row items-center gap-2">
					<header class="h4 grow truncate align-middle font-bold">{region.title}</header>
					<button
						type="button"
						class="variant-filled btn-icon shrink"
						on:click|stopPropagation={() => deleteRegion(region, i)}>X</button
					>
				</div>
				<div class="mt-8 w-full">
					{#await metadata}
						Loading details...
					{:then metadata}
						<table class="table">
							<thead class="table-head">
								<tr class="[&>th]:text-center">
									<th role="columnheader">Teams</th>
									<th role="columnheader">Fields</th>
									<th role="columnheader">Time Slots</th>
								</tr>
							</thead>
							<tbody class="table-body">
								<tr aria-rowindex={i + 1}>
									<td role="gridcell" aria-colindex="1" tabindex="-1">
										{metadata.team_count}
									</td>
									<td role="gridcell" aria-colindex="2" tabindex="-1">
										{metadata.field_count}
									</td>
									<td role="gridcell" aria-colindex="3" tabindex="-1">
										{metadata.time_slot_count}
									</td>
								</tr>
							</tbody>
						</table>
					{/await}
				</div>
			</button>
		{/each}
	</div>
{/if}
