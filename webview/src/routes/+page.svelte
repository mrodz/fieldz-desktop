<script lang="ts">
	import { slide } from 'svelte/transition';
	import ScheduleList from './schedules/ScheduleList.svelte';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h1 mt-2">Fieldz</h1>
	<section>
		<ScheduleList src="home" />
	</section>
</main>
