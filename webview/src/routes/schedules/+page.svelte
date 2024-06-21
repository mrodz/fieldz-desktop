<script lang="ts">
	import { slide } from 'svelte/transition';
	import ScheduleList from './ScheduleList.svelte';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Schedules</h1>

	<div class="my-4">Here is an overview of the schedules you have already created.</div>

	<hr class="hr my-5" />

	<ScheduleList src="route" />
</main>
