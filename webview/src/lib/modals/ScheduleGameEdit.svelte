<script lang="ts">
	import {
		formatDatePretty,
		type CalendarEvent,
		type EditTeamInput,
		type Schedule,
		type ScheduleGame,
		type TeamExtension,
		type TeamGroup
	} from '$lib';
	import { faCircleInfo } from '@fortawesome/free-solid-svg-icons';
	import { ProgressRadial, getModalStore, getToastStore, popup } from '@skeletonlabs/skeleton';
	import { dialog, invoke } from '@tauri-apps/api';
	import { onMount } from 'svelte';
	import Fa from 'svelte-fa';

	export let parent: any;

	const toastStore = getToastStore();
	const modalStore = getModalStore();

	const game: ScheduleGame = $modalStore[0].meta!.game;
	const schedule: Schedule = $modalStore[0].meta!.schedule;
	const event: CalendarEvent = $modalStore[0].meta!.event;
	const onDelete: (event: CalendarEvent) => void | Promise<void> = $modalStore[0].meta!.onDelete;
	const getTeam: (team_id: number) => Promise<TeamExtension> = $modalStore[0].meta!.getTeam;
	const onSwap: () => void = $modalStore[0].meta!.onSwap;

	function close() {
		parent.onClose();
	}

	function deletionClick() {
		onDelete(event);
		close();
	}

	function swapClick() {
		onSwap();
		close();
	}
</script>

<div class="card w-modal p-5">
	<h2 class="h2">Edit reservation</h2>

	<hr class="hr my-5" />

	<div class="flex flex-col">
		<h3 class="h3">
			Viewing:

			{#if typeof game.team_one !== 'number'}
				Empty reservation
			{:else}
				{#await getTeam(game.team_one)}
					Loading team one...
				{:then teamOne}
					{#if typeof game.team_two !== 'number'}
						Practice for {teamOne.team.name}
					{:else}
						{#await getTeam(game.team_two)}
							Loading team two...
						{:then teamTwo}
							{teamOne.team.name} vs {teamTwo.team.name}
						{/await}
					{/if}
				{/await}
			{/if}
		</h3>
		<div>
			Start: {formatDatePretty(new Date(game.start))}
		</div>
		<div>
			End: {formatDatePretty(new Date(game.end))}
		</div>
	</div>

	<hr class="hr my-5" />

	<div class="grid grid-cols-3">
		<button class="variant-outline btn" on:click={deletionClick}>Delete</button>
		<button class="variant-outline btn" on:click={swapClick}>Swap events</button>
		<button class="variant-filled btn mx-1" on:click={close}>Close</button>
	</div>
</div>
