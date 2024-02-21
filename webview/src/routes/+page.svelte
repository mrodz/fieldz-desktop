<script lang="ts">
	import { type Region } from '$lib';
	import RegionList from './regions/RegionList.svelte';
	import { getModalStore, getToastStore } from '@skeletonlabs/skeleton';

	const modalStore = getModalStore();

	let regionList: RegionList;

	function createRegion() {
		modalStore.trigger({
			type: 'component',
			component: 'regionCreate',
			meta: {
				onCreate(region: Region) {
					regionList.addRegionToFrontend(region);
				}
			}
		});
	}
</script>

<main class="p-4">
	<section class="p-4">
		<h2 class="h2">Regions</h2>

		<RegionList bind:this={regionList} />
		<button class="btn variant-filled mx-auto block" on:click={createRegion}> Create Region </button>
	</section>
	<section class="card p-4 mt-8">
		<h2 class="h2">Calendar</h2>
		<i class="mx-auto">Todo...</i>
	</section>
</main>
