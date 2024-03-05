<script lang="ts">
	import { slide } from 'svelte/transition';
	import Groups from './Groups.svelte';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<Groups />
</main>
