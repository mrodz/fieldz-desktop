<script lang="ts">
	import { MAX_GAMES_PER_FIELD_TYPE, MIN_GAMES_PER_FIELD_TYPE, type ReservationType } from '$lib';
	import { faSpinner, faTrash } from '@fortawesome/free-solid-svg-icons';
	import { getModalStore, SlideToggle } from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';

	export let reservation: ReservationType;
	const reservationNameOnLoad = reservation.name;

	let modalStore = getModalStore();

	let dispatch = createEventDispatcher<{
		delete: ReservationType;
		debouncedUpdate: {
			reservation: ReservationType;
			options: {
				nameOnLoad: string;
			};
		};
	}>();

	function onDelete() {
		deleting = true;
		modalStore.trigger({
			type: 'confirm',
			title: 'Please Confirm—This one is really important',
			body: `Deleting a reservation type is PERMANENT! Are you sure you wish to proceed?<br/><br/><b>⚠️ DELETING A FIELD TYPE WILL PERMANENTLY ERASE ALL CALENDAR EVENTS THAT USE THIS FIELD SIZE</b><br/><br/>You will NOT be able to recover "${reservation.name}". Only proceed if you are sure this is what you want. You may have to input a lot of time records again.`,
			response(r) {
				if (r) {
					dispatch('delete', reservation);
				} else {
					deleting = false;
				}
			}
		});
	}

	let reportTimer: NodeJS.Timeout | undefined;
	let pendingPost: boolean = false;

	function requestUpdate() {
		clearTimeout(reportTimer);
		pendingPost = true;
		reportTimer = setTimeout(async () => {
			dispatch('debouncedUpdate', {
				reservation,
				options: {
					nameOnLoad: reservationNameOnLoad
				}
			});
			pendingPost = false;
		}, 1_000);
	}

	function increaseCount() {
		if (reservation.default_sizing < MAX_GAMES_PER_FIELD_TYPE) {
			reservation.default_sizing++;
			requestUpdate();
		}
	}

	function decreaseCount() {
		if (reservation.default_sizing > MIN_GAMES_PER_FIELD_TYPE) {
			reservation.default_sizing--;
			requestUpdate();
		}
	}

	let deleting = false;
</script>

<div class="card p-4 duration-1000" class:animate-pulse={deleting}>
	<div class="grid grid-cols-[auto_1fr] gap-4">
		<div class="grid h-full grid-rows-2 items-center justify-center">
			<input
				class="input m-2 shadow-2xl"
				type="color"
				on:change={() => requestUpdate()}
				bind:value={reservation.color}
			/>
			{#if pendingPost}
				<div class="flex flex-col items-center">
					<Fa class="m-auto inline animate-spin" size="lg" icon={faSpinner} />
					Saving
				</div>
			{:else}
				<button class="btn-icon-md btn-icon m-auto" on:click={() => onDelete()}>
					<Fa class="inline" size="lg" icon={faTrash} />
				</button>
			{/if}
		</div>
		<div class="grid grid-cols-[1fr_auto] gap-2">
			<strong class="my-2">
				<input
					class="w-full border-none bg-transparent"
					bind:value={reservation.name}
					on:keydown={() => requestUpdate()}
					placeholder="Give your classification a name"
				/>
			</strong>
			<div class="flex flex-col items-center">
				Is Practice
				<SlideToggle name="slide" bind:checked={reservation.is_practice} on:change={() => requestUpdate()} />
			</div>
				<textarea
					class="textarea variant-form-material resize-none"
					placeholder="Write a note for yourself to add any additional information you need"
					rows="2"
					on:keydown={() => requestUpdate()}
					bind:value={reservation.description}
				/>
				<div class="max-w-20 text-center">
					Default matches per field space
					<div class="mx-auto grid grid-cols-[1fr_auto_1fr]">
						<button class="btn-icon btn-icon-sm mr-auto" on:click={decreaseCount}>
							-
						</button>
						<div class="mx-2 text-center align-middle leading-loose">
							{reservation.default_sizing}
						</div>
						<button class="btn-icon btn-icon-sm ml-auto" on:click={increaseCount}>
							+
						</button>
					</div>
				</div>
		</div>
	</div>
</div>
