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
		storePopup,
		Avatar,
		getToastStore,
		popup,
		type PopupSettings
	} from '@skeletonlabs/skeleton';
	import { invoke, dialog } from '@tauri-apps/api';
	import { getVersion } from '@tauri-apps/api/app';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	import { computePosition, autoUpdate, offset, shift, flip, arrow } from '@floating-ui/dom';

	import RegionCreate from './region/RegionCreate.svelte';
	import FieldCreate from './fields/FieldCreate.svelte';
	import TeamCreate from './fields/TeamCreate.svelte';
	import RegionEdit from './region/RegionEdit.svelte';
	import TeamEdit from './fields/TeamEdit.svelte';
	import ScheduleEdit from './schedules/ScheduleEdit.svelte';
	import Processing from './scheduler/Processing.svelte';
	import { HAS_DB_RESET_BUTTON } from '$lib';
	import authStore from '$lib/authStore';
	import { initializeApp } from 'firebase/app';
	import {
		initializeAuth,
		onAuthStateChanged,
		browserPopupRedirectResolver,
		browserLocalPersistence,
		getAuth
	} from 'firebase/auth';
	import { onMount } from 'svelte';
	import Fa from 'svelte-fa';
	import { faSignIn } from '@fortawesome/free-solid-svg-icons';

	initializeStores();

	storePopup.set({ computePosition, autoUpdate, offset, shift, flip, arrow });

	const toastStore = getToastStore();

	onMount(() => {
		/*
		 * This data is meant to be public; it's okay to be checked in to source control.
		 *
		 * For Google Cloud projects, `apiKey` is really just a service identifier.
		 *
		 * FYI I've restricted the scopes for this app already.
		 */
		const firebaseConfig = {
			apiKey: 'AIzaSyBl_mPnweSBKiiB-av9pG_ktomlKv8vbvw',
			authDomain: 'fieldmasterapp.firebaseapp.com',
			projectId: 'fieldmasterapp',
			storageBucket: 'fieldmasterapp.appspot.com',
			messagingSenderId: '496161054017',
			appId: '1:496161054017:web:275f97a408237dee6c15c0',
			measurementId: 'G-HP1ZMG0ZV0'
		};

		const firebaseApp = initializeApp(firebaseConfig);

		const firebaseAuth = initializeAuth(firebaseApp, {
			popupRedirectResolver: browserPopupRedirectResolver,
			persistence: browserLocalPersistence
		});

		onAuthStateChanged(firebaseAuth, (user) => {
			if (user) {
				toastStore.trigger({
					message: `Welcome, ${user.displayName ?? 'Guest'}`,
					background: 'variant-filled-success'
				});
			}

			authStore.set({
				isLoggedIn: user !== null,
				user: user ?? undefined,
				firebaseControlled: true
			});
		});
	});

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
		},
		scheduleEdit: {
			ref: ScheduleEdit
		},
		processingSchedule: {
			ref: Processing
		}
	};

	async function resetDatabase() {
		await invoke('db_migrate_up_down');
	}

	async function signIn() {
		const thisUrl = $page.url.pathname;
		goto(`/login?next=${thisUrl}`);
	}

	async function signOut() {
		try {
			await getAuth().signOut();

			toastStore.trigger({
				message: "You signed out! You'll have to sign in again to use our servers.",
				background: 'variant-filled-success'
			});
		} catch (e) {
			dialog.message(JSON.stringify(e), {
				title: 'Could not sign out',
				type: 'error'
			});
		}
	}

	const avatarClick = {
		event: 'click',
		target: 'avatarClick',
		placement: 'bottom'
	} satisfies PopupSettings;
</script>

<Toast />
<Modal components={modalRegistry} />

<AppShell slotSidebarLeft="bg-surface-500/5 w-56 p-4">
	<svelte:fragment slot="sidebarLeft">
		<nav class="list-nav">
			<ul>
				<li><a href="/">Regions</a></li>
				<li><a href="/groups">Groups</a></li>
				<li><a href="/field-types">Field Types</a></li>
				<li><a href="/scheduler">Scheduler</a></li>
				<li><a href="/schedules">Schedules</a></li>
			</ul>
		</nav>
		{#await getVersion() then version}
			<div class="absolute bottom-0 m-4">
				v{version}
			</div>
		{/await}
	</svelte:fragment>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead">
				<LightSwitch />

				{#if HAS_DB_RESET_BUTTON}
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
				{/if}
			</svelte:fragment>

			<svelte:fragment slot="trail">
				{#if $authStore.user !== undefined}
					<div use:popup={avatarClick}>
						{#if $authStore.user.photoURL}
							<Avatar
								cursor="cursor-pointer"
								border="border-4 border-surface-300-600-token hover:!border-primary-500"
								width="w-16"
								src={$authStore.user.photoURL}
							/>
						{:else if $authStore.user.displayName}
							<Avatar
								cursor="cursor-pointer"
								border="border-4 border-surface-300-600-token hover:!border-primary-500"
								width="w-16"
								initials={$authStore.user.displayName}
							/>
						{:else}
							<Avatar
								cursor="cursor-pointer"
								border="border-4 border-surface-300-600-token hover:!border-primary-500"
								width="w-16"
								initials="??"
							/>
						{/if}
					</div>

					<div class="card variant-filled-primary w-96 p-4" data-popup="avatarClick">
						<p>
							Hi, {$authStore.user.displayName ?? 'Guest'}.
						</p>

						<p class="mt-2">
							Thank you for signing in! You can generate schedules with this account.
						</p>

						{#if $authStore.user.email}
							<p class="mt-2">
								If we need to contact you, the email we have on file is
								{$authStore.user.email}.
							</p>
						{/if}

						<button class="variant-filled btn mt-2" on:click={signOut}>Sign Out</button>
					</div>
				{:else}
					<button class="btn" on:click={signIn}>
						<span class="mr-2"> Sign In </span>

						<Fa icon={faSignIn} />
					</button>
				{/if}
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>
	<slot />
</AppShell>
