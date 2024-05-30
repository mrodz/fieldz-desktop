<script lang="ts">
	import {
		GoogleAuthProvider,
		TwitterAuthProvider,
		signInWithPopup,
		getAuth,
		GithubAuthProvider,
		OAuthProvider,
		signInWithRedirect
	} from 'firebase/auth';
	import { goto } from '$app/navigation';
	import { slide } from 'svelte/transition';
	import { getToastStore } from '@skeletonlabs/skeleton';
	import GoogleIcon from './GoogleIcon.svelte';
	import TwitterIcon from './TwitterIcon.svelte';
	import GitHubIcon from './GitHubIcon.svelte';
	import MicrosoftIcon from './MicrosoftIcon.svelte';
	import { type } from '@tauri-apps/api/os';
	import { githubLogin, googleLogin, twitterLogin } from '$lib/auth';
	import authStore from '$lib/authStore';

	const queryParams = new URLSearchParams(window.location.search);
	const next = queryParams.get('next') ?? '/';

	const toastStore = getToastStore();

	const signInFunction = type().then((type) => {
		switch (type) {
			/*
			 * The webview on OSX does not support browser popups.
			 * Why, you might ask?
			 * No one knows :(
			 */
			case 'Darwin':
				return signInWithRedirect;
			default:
				return signInWithPopup;
		}
	});

	function duplicatedMessage(error: any) {
		if (
			typeof error === 'object' &&
			'code' in error &&
			error.code === 'auth/account-exists-with-different-credential'
		) {
			toastStore.trigger({
				message:
					'You have used a different method of authentication in the past! Please try a different authentication platform.',
				background: 'variant-filled-warning',
				autohide: false,
			});
		} else {
			console.error(error);
		}
	}

	async function google() {
		try {
			await googleLogin(
				async (credential) => {
					console.log($authStore.user, credential);
					goto(next);
				},
				(e) => {
					console.warn(e);
					duplicatedMessage(e);
				}
			);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}

	async function twitter() {
		try {
			await twitterLogin(async (credential) => {
				console.log($authStore.user, credential);
				goto(next);
			});
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}

	async function github() {
		try {
			await githubLogin(
				async (credential) => {
					console.log($authStore.user, credential);
					goto(next);
				},
				(e) => {
					console.warn(e);
					duplicatedMessage(e);
				}
			);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}

	async function microsoft() {
		try {
			const provider = new OAuthProvider('microsoft.com');
			provider.setCustomParameters({
				prompt: 'select_account'
			});

			const userCredential = await (await signInFunction)(getAuth(), provider);

			console.log(userCredential);

			goto(next);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}
</script>

<main in:slide={{ axis: 'x' }} out:slide={{ axis: 'x' }} class="p-4">
	<h1 class="h2">Login</h1>

	<button class="variant-filled btn my-4" on:click={() => history.back()}>&laquo;&nbsp; Back</button
	>

	<div class="logo-cloud grid-cols-1 gap-0.5 md:grid-cols-2 2xl:grid-cols-4">
		<button class="logo-item card-hover" on:click={google}>
			<GoogleIcon class="mr-4 w-12" />
			Sign In
		</button>
		<button disabled class="logo-item cursor-not-allowed bg-gray-400" on:click={twitter}>
			<TwitterIcon class="mr-4 w-12" />
			Sign In
		</button>
		<button class="logo-item card-hover" on:click={github}>
			<GitHubIcon class="mr-4 w-12" />
			Sign In
		</button>
		<button disabled class="logo-item cursor-not-allowed bg-gray-400" on:click={microsoft}>
			<MicrosoftIcon class="mr-4 w-12" />
			Sign In
		</button>
	</div>

	<div class="card mt-4 bg-yellow-300 p-4 text-center mx-auto md:w-2/3 xl:w-1/3">
		<header class="card-header font-bold">Temporary Notice</header>

		<p>
			Authentication via Twitter (X) and Microsoft is disabled for the moment. We apologize for the
			inconvenience and are working to integrate these platforms for the next release.
		</p>
	</div>

	<hr class="hr my-10" />

	<div>
		<h2 class="h3 mb-2">Why do I need to sign in?</h2>

		<p class="mb-2">
			You can use Fieldz freely as much as you'd like &mdash; all data is stored on your local
			machine. We require authentication for any server-based actions, like creating a schedule from
			the inputs you gave the app.
		</p>

		<p>
			You need to be signed in to talk to our servers because creating a schedule is very intensive
			for our CPUs in the cloud. In order to prevent bad actors from wasting our resources, using a
			Fieldz account lets us monitor how many schedules are being created to prevent misuse. Thank
			you for your cooperation!
		</p>
	</div>
</main>
