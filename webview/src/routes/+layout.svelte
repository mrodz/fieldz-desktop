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

	import { computePosition, autoUpdate, offset, shift, flip, arrow } from '@floating-ui/dom';

	import RegionCreate from './region/RegionCreate.svelte';
	import FieldCreate from './fields/FieldCreate.svelte';

	initializeStores();

	storePopup.set({ computePosition, autoUpdate, offset, shift, flip, arrow });

	const modalRegistry: Record<string, ModalComponent> = {
		regionCreate: {
			ref: RegionCreate
		},
		fieldCreate: {
			ref: FieldCreate
		}
	};
</script>

<Toast />
<Modal components={modalRegistry} />

<AppShell slotSidebarLeft="bg-surface-500/5 w-56 p-4">
	<svelte:fragment slot="sidebarLeft">
		<nav class="list-nav">
			<ul>
				<li><a href="/">Home</a></li>
			</ul>
		</nav>
	</svelte:fragment>
	<svelte:fragment slot="header">
		<AppBar>
			<LightSwitch />
		</AppBar>
	</svelte:fragment>
	<slot />
</AppShell>
