<script lang="ts">
	import { slide } from 'svelte/transition';
	import ReservationType from './ReservationType.svelte';
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h1 class="h2 m-4">Field Types</h1>

	<section class="m-4 [&>*]:my-4">
		<p>
			Here is where you will specify the different types of reservations you can create.
		</p>

		<p>
			For example, a team for six year olds will likely play on
			a different sized field than a team of older teenagers.
		</p>
	</section>

	<section>
		<ReservationType reservation={{ id: 1, name: 'small field', color: '#ffffff' }} />
	</section>
</main>