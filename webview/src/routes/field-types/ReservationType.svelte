<script lang="ts">
	import type { ReservationType } from '$lib';
	import { faTrash } from '@fortawesome/free-solid-svg-icons';
	import { getModalStore } from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';

	export let reservation: ReservationType;

	let modalStore = getModalStore();

	let dispatch = createEventDispatcher<{
		delete: ReservationType;
	}>();

	let colorValue: string;

	function onDelete() {
		modalStore.trigger({
			type: 'confirm',
			response(r) {
				if (r) {
					dispatch('delete', reservation);
				}
			}
		});
	}
</script>

<div class="card p-4">
	<div class="grid grid-cols-[auto_1fr_auto] gap-4">
		<div class="flex flex-col h-full justify-center items-center">
			<input class="input shadow-2xl m-2" type="color" bind:value={colorValue} />
			<button class="icon-btn btn-icon-sm" on:click={() => onDelete()}>
				<Fa class="inline" size="lg" icon={faTrash} />
			</button>
		</div>
		<div>
			<strong class="my-2">
				<input
					class="w-full border-none bg-transparent"
					value={reservation.name}
					placeholder="Give your classification a name"
				/>
			</strong>
			<div class="my-2">
				<textarea
					class="textarea variant-form-material resize-none"
					placeholder="No description"
					rows="2"
					{...reservation.description !== undefined ? { value: reservation.description } : {}}
				/>
			</div>
		</div>
	</div>
</div>
