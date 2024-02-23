<script lang="ts">
	import { type Region } from '$lib';
	import { slide } from 'svelte/transition';
	import RegionList from './region/RegionList.svelte';
	import { getModalStore, InputChip, popup } from '@skeletonlabs/skeleton';
	import Fa from 'svelte-fa';
	import { faCircleInfo } from '@fortawesome/free-solid-svg-icons';

	const modalStore = getModalStore();

	let regionList: RegionList;

	function createRegion() {
		modalStore.trigger({
			type: 'component',
			component: 'regionCreate',
			meta: {
				onCreate(region: Region) {
					regionList.addRegionToFrontend(region);
				}
			}
		});
	}

	let groups: string[] = [];
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<section class="p-4">
		<h2 class="h2">
			Groups
			<span
				aria-haspopup="dialog"
				use:popup={{
					event: 'hover',
					target: 'groupsPopup'
				}}
			>
				<Fa class="inline" size="xs" icon={faCircleInfo} />
			</span>
		</h2>

		<div class="card arrow w-72 p-4 shadow-xl [&>*]:pointer-events-none" data-popup="groupsPopup">
			<div>
				<p>
					<strong>What is this input?</strong>
				</p>
				<p>
					Here is where you create labels you'll use to group teams
					<u>across regions</u>. Teams that you create in any region can have many grouping labels.
				</p>
				<br />
				<p>Some use cases might be:</p>
				<ul class="list">
					<li>&bull; Age Groups (u8, u10, u12, etc)</li>
					<li>&bull; Extras or All Stars</li>
					<li>&bull; Tournament</li>
				</ul>
				<br/>
			</div>
			<div class="arrow bg-surface-100-800-token" />
		</div>

		<InputChip bind:value={groups} class="mt-4" name="groups" placeholder="Start typing a label, then hit enter" {...$$restProps} />
	</section>
	<section class="p-4">
		<h2 class="h2">Regions</h2>

		<RegionList bind:this={regionList} />
		<button class="variant-filled btn mx-auto block" on:click={createRegion}>
			Create Region
		</button>
	</section>
</main>
