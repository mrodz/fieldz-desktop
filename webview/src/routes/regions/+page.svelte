<script lang="ts">
	import { slide } from 'svelte/transition';
	import RegionList from './RegionList.svelte';
	import CreateRegionButton from './CreateRegionButton.svelte';

	let regionList: RegionList;
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h2 class="h2">Regions</h2>

	<RegionList bind:this={regionList} />

	<CreateRegionButton {regionList} />
</main>
