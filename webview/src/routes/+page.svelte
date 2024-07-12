<script lang="ts">
	import { slide } from 'svelte/transition';
	import ScheduleList from './schedules/ScheduleList.svelte';
	import Groups from './groups/Groups.svelte';
	import FieldTypes from './field-types/FieldTypes.svelte';
	import RegionList from './regions/RegionList.svelte';
	import CreateRegionButton from './regions/CreateRegionButton.svelte';

	let regionList: RegionList;
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h1 class="h1 m-8">Home</h1>

	<section class="card sticky top-0 my-4 rounded-none border-none px-8 py-4 shadow-none ring-0">
		<header class="h4 mb-2">Jump To</header>
		<div class="flex flex-row gap-4">
			<a href="#groups" class="chip variant-filled-primary">Groups</a>
			<a href="#sizes" class="chip variant-filled-primary">Field Sizes</a>
			<a href="#regions" class="chip variant-filled-primary">Regions</a>
			<a href="#schedules" class="chip variant-filled-primary">Schedules</a>
		</div>
	</section>

	<div class="p-4 [&>section]:my-8 [&>section]:p-8">
		<section id="groups">
			<h2 class="h2">Team Groupings</h2>
			<Groups />
		</section>

		<section id="sizes">
			<h2 class="h2">Field Types/Sizes</h2>
			<FieldTypes />
		</section>

		<section id="regions">
			<h2 class="h2">Regions</h2>
			<RegionList bind:this={regionList} />
			<CreateRegionButton {regionList} />
		</section>

		<section id="schedules">
			<h2 class="h2">Schedules</h2>
			<ScheduleList src="home" />
		</section>


	</div>
</main>
