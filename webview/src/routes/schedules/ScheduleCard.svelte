<script lang="ts">
	import type { Schedule } from '$lib';
	import { faCaretDown } from '@fortawesome/free-solid-svg-icons';
	import { popup, type PopupSettings } from '@skeletonlabs/skeleton';
	import { createEventDispatcher } from 'svelte';
	import Fa from 'svelte-fa';

	export let schedule: Schedule;

	const dispatch = createEventDispatcher<{}>();

	const DATE_OPTIONS: Intl.DateTimeFormatOptions = {
		hour: 'numeric',
		minute: 'numeric'
	};

	const popupClick: PopupSettings = {
		event: 'click',
		target: 'popupClick',
		placement: 'top'
	};
</script>

<div class="card m-4 grid grid-cols-[1fr_auto]">
	<div>
		<header class="card-header">
			<strong>
				{schedule.name}
			</strong>
		</header>
		<section class="p-4">
			<div>
				<div>
					Created: {new Date(schedule.created).toLocaleDateString('en-US', DATE_OPTIONS)}
				</div>
				<div>
					Last Edited: {new Date(schedule.last_edit).toLocaleDateString('en-US', DATE_OPTIONS)}
				</div>
			</div>
		</section>
	</div>
	<div class="align-center flex">
		<button class="btn-icon" use:popup={popupClick}>
			<Fa icon={faCaretDown} />
		</button>
	</div>
</div>
