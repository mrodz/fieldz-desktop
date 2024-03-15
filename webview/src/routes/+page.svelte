<script lang="ts">
	import { type Region } from '$lib';
	import { slide } from 'svelte/transition';
	import RegionList from './region/RegionList.svelte';
	import { getModalStore, getToastStore } from '@skeletonlabs/skeleton';

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	let regionList: RegionList;

	function createRegion() {
		modalStore.trigger({
			type: 'component',
			component: 'regionCreate',
			meta: {
				onCreate(region: Region) {
					toastStore.trigger({
						message: `Created new region: "${region.title}"`,
						background: 'variant-filled-success'
					});
					regionList.addRegionToFrontend(region);
				}
			}
		});
	}
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<!--
	<Groups />

	<hr class="!border-t-4 my-4" />
	-->

	<section class="p-4">
		<h2 class="h2">Regions</h2>

		<RegionList bind:this={regionList} />
		<button class="variant-filled btn mx-auto block" on:click={createRegion}>
			Create Region
		</button>
	</section>
</main>
