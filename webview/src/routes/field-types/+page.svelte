<script lang="ts">
	import { slide } from 'svelte/transition';
	import { onMount } from 'svelte';
	import ReservationTypeComponent from './ReservationType.svelte';
	import { type ReservationType, type CreateReservationTypeInput, randomCalendarColor } from '$lib';
	import { invoke, dialog } from '@tauri-apps/api';
	import { ProgressRadial } from '@skeletonlabs/skeleton';

	let reservations: ReservationType[] | undefined;

	onMount(async () => {
		try {
			reservations = await invoke<ReservationType[]>('get_reservation_types');
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not load reservation types',
				type: 'error'
			});
		}
	});

	async function newType() {
		try {
			let name = 'New Reservation Type';

			if (reservations?.length ?? 0 !== 0) {
				const last = reservations?.findLast((reservationType) =>
					/^New Reservation Type(\s\(\d+\))?$/.test(reservationType.name)
				);

				if (last !== undefined) {
					if (last.name === name) {
						name += ' (1)';
					} else {
						const lastNumberString = last.name.slice(22, last.name.length - 1);
						const lastNumber = Number(lastNumberString);
						name += ` (${lastNumber + 1})`;
					}
				}
			}

			const input = {
				name,
				color: randomCalendarColor()
			} satisfies CreateReservationTypeInput;

			const type = await invoke<ReservationType>('create_reservation_type', { input });

			reservations?.push(type);
			reservations = reservations;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not create reservation type',
				type: 'error'
			});
		}
	}

	async function deleteType(reservationType: ReservationType, index: number) {
		try {
			await invoke<ReservationType>('delete_reservation_type', { id: reservationType.id });

			reservations?.splice(index, 1);
			reservations = reservations;
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not delete reservation type',
				type: 'error'
			});
		}
	}

	async function updateType(
		reservationType: ReservationType,
		options?: {
			nameOnLoad: string;
		}
	) {
		try {
			if (reservationType.name.length === 0) {
				reservationType.name = options?.nameOnLoad ?? 'New Reservation Type (?)';
			}

			await invoke<ReservationType>('update_reservation_type', { reservationType });
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not save changes to reservation type',
				type: 'error'
			});
		}
	}
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }}>
	<h1 class="h2 m-4">Field Types</h1>

	<section class="m-4 [&>*]:my-4">
		<p>Here is where you will specify the different types of reservations you can create.</p>

		<p>
			For example, a team for six year olds will likely play on a different sized field than a team
			of older teenagers.
		</p>
	</section>

	<section class="m-4">
		{#if reservations === undefined}
			<ProgressRadial />
		{:else if reservations.length !== 0}
			<div class="grid grid-cols-1 gap-2 lg:grid-cols-2">
				{#each reservations as reservation, i}
					<ReservationTypeComponent
						{reservation}
						on:delete={(e) => deleteType(e.detail, i)}
						on:debouncedUpdate={(e) => updateType(e.detail.reservation, e.detail.options)}
					/>
				{/each}
			</div>

			<hr class="hr my-5" />

			<button class="variant-filled btn" on:click={() => newType()}>+ New Type</button>
		{:else}
			<div class="card m-4 p-4 text-center">
				<i class="my-4 block">You haven't created any reservation types yet</i>
				<hr class="hr my-5" />
				<button class="variant-filled btn" on:click={() => newType()}>+ New Type</button>
			</div>
		{/if}
	</section>
</main>
