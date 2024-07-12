<script lang="ts">
	import { dialog, invoke } from "@tauri-apps/api";
	import { default as ReservationTypeComponent } from "./ReservationType.svelte";
	import { onMount } from "svelte";
	import { randomCalendarColor, type CreateReservationTypeInput, type ReservationType } from "$lib";
	import { ProgressRadial } from "@skeletonlabs/skeleton";

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

<section class="m-4 flex flex-col items-center">
	{#if reservations === undefined}
		<ProgressRadial />
	{:else if reservations.length !== 0}
		<div class="w-full xl:w-4/5 grid grid-cols-1 gap-10 lg:grid-cols-2 mb-4">
			{#each reservations as reservation, i}
				<ReservationTypeComponent
					{reservation}
					on:delete={(e) => deleteType(e.detail, i)}
					on:debouncedUpdate={(e) => updateType(e.detail.reservation, e.detail.options)}
				/>
			{/each}
		</div>

		<button class="variant-filled btn mx-auto" on:click={() => newType()}>Create Reservation Type</button>
	{:else}
		<div class="card m-4 p-4 text-center">
			<i class="my-4 block">You haven't created any reservation types yet</i>
			<button class="variant-filled btn mx-auto" on:click={() => newType()}>Create Reservation Type</button>
		</div>
	{/if}
</section>
