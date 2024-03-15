<script lang="ts">
	import '../app.pcss';
	import {
		AppShell,
		AppBar,
		initializeStores,
		LightSwitch,
		Modal,
		type ModalComponent,
		Toast,
		storePopup
	} from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';
	import { computePosition, autoUpdate, offset, shift, flip, arrow } from '@floating-ui/dom';

	import RegionCreate from './region/RegionCreate.svelte';
	import FieldCreate from './fields/FieldCreate.svelte';
	import TeamCreate from './fields/TeamCreate.svelte';
	import RegionEdit from './region/RegionEdit.svelte';
	import TeamEdit from './fields/TeamEdit.svelte';

	initializeStores();

	storePopup.set({ computePosition, autoUpdate, offset, shift, flip, arrow });

	const modalRegistry: Record<string, ModalComponent> = {
		regionCreate: {
			ref: RegionCreate
		},
		fieldCreate: {
			ref: FieldCreate
		},
		teamCreate: {
			ref: TeamCreate
		},
		regionEdit: {
			ref: RegionEdit
		},
		teamEdit: {
			ref: TeamEdit
		}
	};

	async function resetDatabase() {
		await invoke('db_migrate_up_down');
	}
</script>

<Toast />
<Modal components={modalRegistry} />

<AppShell slotSidebarLeft="bg-surface-500/5 w-56 p-4">
	<svelte:fragment slot="sidebarLeft">
		<nav class="list-nav">
			<ul>
				<li><a href="/">Home</a></li>
				<li><a href="/groups">Groups</a></li>
				<li><a href="/scheduler">Scheduler</a></li>
			</ul>
		</nav>
	</svelte:fragment>
	<svelte:fragment slot="header">
		<AppBar>
			<div class="flex flex-row items-center justify-center">
				<LightSwitch />
				<button
					class="variant-outline btn ml-4"
					on:click|preventDefault={async () => {
						await resetDatabase();
						window.location.replace('/');
						dialog.message("The app's data was wiped, and the database's schema was refreshed.", {
							title: 'Database reset complete',
							type: 'info'
						});
					}}
				>
					Reset Database &mdash; Destructive
				</button>
			</div>
		</AppBar>
	</svelte:fragment>
	<slot />
</AppShell>
