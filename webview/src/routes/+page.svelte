<script lang="ts">
	import { type Region } from '$lib';
	import { slide } from 'svelte/transition';
	import RegionList from './region/RegionList.svelte';
	import { getModalStore, InputChip, popup } from '@skeletonlabs/skeleton';
	import Fa from 'svelte-fa';
	import { faCircleInfo } from '@fortawesome/free-solid-svg-icons';

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

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<section class="p-4">
		<h2 class="h2">Regions</h2>

		<RegionList bind:this={regionList} />
		<button class="variant-filled btn mx-auto block" on:click={createRegion}>
			Create Region
		</button>
	</section>
</main>
