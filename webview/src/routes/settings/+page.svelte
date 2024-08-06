<script lang="ts">
	import { slide } from 'svelte/transition';
	import { onMount } from 'svelte';
	import { invoke, dialog } from '@tauri-apps/api';
	import { listen } from '@tauri-apps/api/event';
	import { ProgressRadial, getModalStore, getToastStore } from '@skeletonlabs/skeleton';
	import profileStore from '$lib/profileStore';
	import profileListStore from '$lib/profileListStore';
	import { handleProfileCreationError } from '$lib';

	let modalStore = getModalStore();
	let toastStore = getToastStore();

	function renameProfile(profileName: string) {
		modalStore.trigger({
			type: 'prompt',
			title: 'Enter profile name',
			body: `You are renaming profile ${profileName}. Give it a distinct name, with a max length of 64 characters, that uses letters, numbers, "_", "-", and spaces.`,
			async response(r: false | string | undefined) {
				if (!r) return;

				try {
					const newProfile = await invoke<string>('rename_profile', {
						profileName,
						newName: r
					});

					$profileListStore = $profileListStore.then((profileListStore) =>
						profileListStore.map((bundle) => {
							if (bundle[0] === profileName) {
								bundle[0] = newProfile;
							}
							return bundle;
						})
					);

					toastStore.trigger({
						message: `Renamed profile "${profileName}" to "${newProfile}"`,
						background: 'variant-filled-success'
					});
				} catch (e) {
					handleProfileCreationError(e, toastStore, dialog.message);
				}
			}
		});
	}

	function deleteProfile(profileName: string) {
		modalStore.trigger({
			type: 'confirm',
			title: 'Are you sure you want to delete this profile?',
			body: 'This action is PERMANENT and will PERMANENTLY DELETE THE DATABASE FOR THIS PROFILE. There is NO WAY TO RECOVER THIS PROFILE.',
			response(r) {
				if (!r) return;
				modalStore.trigger({
					type: 'confirm',
					title: '⚠️⚠️ Are you REALLY, REALLY sure you want to delete this profile?',
					body: 'Seriously. Last chance to back out. This action is PERMANENT and will PERMANENTLY DELETE THE DATABASE FOR THIS PROFILE. There is NO WAY TO RECOVER THIS PROFILE.',
					async response(r) {
						if (!r) return;
						try {
							await invoke('delete_profile', {
								profileName
							});
							$profileListStore = $profileListStore.then((profiles) => {
								return profiles.filter(([profile, _]) => profile !== profileName);
							});
							toastStore.trigger({
								message: `Deleted profile: ${profileName}`,
								background: 'variant-filled-success'
							});
						} catch (e) {
							dialog.message(JSON.stringify(e), {
								type: 'error',
								title: 'Could not delete profile'
							});
						}
					}
				});
			}
		});
	}

	let profileName: string | undefined | null;

	$profileStore.name.then((done) => (profileName = done));
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Settings</h1>

	<button class="variant-filled btn my-4" on:click={() => history.back()}>
		&laquo;&nbsp; Back
	</button>

	<div class="flex flex-col gap-4">
		<section class="card p-4">
			<h2 class="h3">Profiles</h2>
			<p>
				Rename and delete your custom profiles here. You may not delete the active profile or the
				default profile.
			</p>

			<hr class="hr my-2" />

			{#await $profileListStore}
				<ProgressRadial width="w-8" />
			{:then profiles}
				<ul class="flex flex-col gap-8 sm:flex-row">
					{#each profiles ?? [] as [profile, metadata]}
						<li class="mx-4 my-4 flex flex-col gap-4">
							<header>
								{profile} &mdash; {metadata.size ?? 0} bytes
							</header>
							<div class="grid grid-cols-2">
								<button
									class="variant-filled btn"
									disabled={profileName === profile}
									on:click={() => renameProfile(profile)}>Rename</button
								>
								<button
									class="variant-filled btn"
									disabled={profileName === profile}
									on:click={() => deleteProfile(profile)}>Delete</button
								>
							</div>
						</li>
					{:else}
						<li>You haven't created any custom profiles</li>
					{/each}
				</ul>
			{:catch}
				Could not load profiles.
			{/await}
		</section>
	</div>
</main>
