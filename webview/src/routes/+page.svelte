<script lang="ts">
	import { slide } from 'svelte/transition';
	import ScheduleList from './schedules/ScheduleList.svelte';
	import Groups from './groups/Groups.svelte';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h1 mt-2">Fieldz</h1>

	<hr class="hr my-5" />

	<section class="card sticky my-4 p-4">
		<header class="h4 mb-2">Jump To</header>
		<div class="flex flex-row gap-4">
			<a href="#schedules" class="chip variant-filled-primary">Schedules</a>
			<a href="#schedules" class="chip variant-filled-primary">Groups</a>
		</div>
	</section>

	<hr class="hr my-5" />

	<section id="groups" class="my-8">
		<h2 class="h2">Team Groupings</h2>
		<Groups />
	</section>

	<section id="groups" class="my-8">
		<h2 class="h2">Field Types</h2>
		
	</section>

	<section id="schedules" class="my-8">
		<h2 class="h2">Schedules</h2>
		<ScheduleList src="home" />
	</section>

	
</main>
