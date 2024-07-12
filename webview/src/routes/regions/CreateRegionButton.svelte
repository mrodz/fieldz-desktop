<script lang="ts">
	import type { Region } from "$lib";
	import { getModalStore, getToastStore } from "@skeletonlabs/skeleton";
	import RegionList from "./RegionList.svelte";

	export let regionList: RegionList;

	const toastStore = getToastStore();
	const modalStore = getModalStore();

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

<button class="variant-filled btn mx-auto block" on:click={createRegion}> Create Region </button>
