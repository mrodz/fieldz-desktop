<script lang="ts">
	import { slide } from 'svelte/transition';
	import ScheduleList from './schedules/ScheduleList.svelte';
	import Groups from './groups/Groups.svelte';
	import FieldTypes from './field-types/FieldTypes.svelte';
	import RegionList from './regions/RegionList.svelte';
	import CreateRegionButton from './regions/CreateRegionButton.svelte';

	let regionList: RegionList;
</script>

<main id="fieldz-home" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h1 class="h1 m-8">Home</h1>

	<section class="card sticky top-0 my-4 rounded-none border-none px-8 py-4 shadow-none ring-0 z-20">
		<header class="h4 mb-2">Jump To</header>
		<div class="flex flex-row gap-4">
			<a href="#groups" class="chip variant-filled-primary">Groups</a>
			<a href="#sizes" class="chip variant-filled-primary">Field Sizes</a>
			<a href="#regions" class="chip variant-filled-primary">Regions</a>
			<a href="#schedules" class="chip variant-filled-primary">Schedules</a>
		</div>
	</section>

	<div class="p-4">
		<section id="groups">
			<h2 class="h2 mb-16">Team Groupings</h2>
			<Groups />
		</section>

		<section id="sizes">
			<h2 class="h2">Field Types/Sizes</h2>
			<FieldTypes />
		</section>

		<section id="regions">
			<h2 class="h2 mb-8">Regions</h2>
			<RegionList bind:this={regionList} />
			<CreateRegionButton {regionList} />
		</section>

		<section id="schedules">
			<h2 class="h2 mb-8">Schedules</h2>
			<ScheduleList src="home" />
		</section>
	</div>
</main>

<style>
	#fieldz-home {
		counter-reset: section-counter;
	}

	section {
		margin: 2rem 0;
		padding: 2rem;
	}

	section > h2 {
		position: relative;
	}

	section > h2::after {
		counter-increment: section-counter;
		content: counter(section-counter);
		position: absolute;
		right: 40px;
		top: 50%;
		transform: translateY(-50%);
		width: 50px;
		height: 50px;
		line-height: 50px;
		border-radius: 50%;
		text-align: center;
		font-weight: bold;
	}

	section#groups > h2::after {
		background-color: #db4d34;
	}

	section#sizes > h2::after {
		background-color: #f5a208;
	}

	section#regions > h2::after {
		background-color: #26af5d;
	}

	section#schedules > h2::after {
		background-color: #08c6f5;
	}
</style>
