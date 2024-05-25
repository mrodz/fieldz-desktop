<script lang="ts">
	import { slide } from 'svelte/transition';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Scheduler</h1>

	<button class="variant-filled btn my-4" on:click={() => history.back()}>
		&laquo;&nbsp; Back
	</button>
</main>
