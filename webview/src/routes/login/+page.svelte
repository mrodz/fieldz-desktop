<script lang="ts">
	import {
		GoogleAuthProvider,
		TwitterAuthProvider,
		signInWithPopup,
		getAuth,
		signInWithRedirect,
		GithubAuthProvider
	} from 'firebase/auth';
	import { goto } from '$app/navigation';
	import { getToastStore } from '@skeletonlabs/skeleton';

	const queryParams = new URLSearchParams(window.location.search);
	const next = queryParams.get('next') ?? '/';

	const toastStore = getToastStore();

	function duplicatedMessage(error: any) {
		if ('code' in error && error.code === 'auth/account-exists-with-different-credential') {
			toastStore.trigger({
				message:
					'You have used a different method of authentication in the past! Please try a different authentication platform.',
				background: 'variant-filled-warning',
				timeout: 10_000
			});
		}
	}

	async function google() {
		try {
			const provider = new GoogleAuthProvider();

			provider.addScope('https://www.googleapis.com/auth/userinfo.email');

			const userCredential = await signInWithPopup(getAuth(), provider);

			console.log(userCredential);

			goto(next);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}

	async function twitter() {
		try {
			const provider = new TwitterAuthProvider();

			const userCredential = await signInWithRedirect(getAuth(), provider);

			console.log(userCredential);

			goto(next);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}

	async function github() {
		try {
			const provider = new GithubAuthProvider();

			const userCredential = await signInWithPopup(getAuth(), provider);

			console.log(userCredential);

			goto(next);
		} catch (e) {
			console.warn(e);
			duplicatedMessage(e);
		}
	}
</script>

<div>
	<div class="logo-cloud grid-cols-1 gap-0.5 xl:grid-cols-2 2xl:grid-cols-4">
		<button on:click={google}>Sign In with Google</button>
		<button on:click={twitter}>Sign In with Twitter</button>
		<button on:click={github}>Sign In with GitHub</button>
	</div>
</div>
