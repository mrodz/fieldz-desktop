<script lang="ts">
	import { slide } from 'svelte/transition';
	import Groups from './Groups.svelte';
</script>

<main class="p-4" in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h2 class="h2">Groups</h2>

	<div class="[&>*]:my-4">
		<p>
			Here is where you create labels you'll use to group teams
			<strong>across regions</strong>. Teams that you create in any region can have many grouping
			labels.
		</p>
		<p>
			Try to make labels as generic as possible. Once you've inputted all of your data, you can
			create scheduling targets to have specific groups play each other.
		</p>
		<p>Some use cases might be:</p>
		<ul class="list">
			<li>&bull; Age Groups (u8, u10, u12, etc)</li>
			<li>&bull; Boys/Girls</li>
			<li>&bull; Extras or All Stars</li>
			<li>&bull; Tournament Brackets</li>
		</ul>
	</div>

	<Groups />
</main>
