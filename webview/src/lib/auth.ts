import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import {
	getAuth,
	GithubAuthProvider,
	GoogleAuthProvider,
	signInWithCredential,
	TwitterAuthProvider,
	type UserCredential
} from 'firebase/auth';
import type { OAuthAccessTokenExchange } from '$lib';
import { env } from '$env/dynamic/public';

async function googleSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	const accessToken = new URLSearchParams(url.hash.substring(1)).get('access_token');
	if (!accessToken) {
		return Promise.reject('Missing `access_token`');
	}

	try {
		const auth = getAuth();
		const credential = GoogleAuthProvider.credential(null, accessToken);
		return signInWithCredential(auth, credential);
	} catch (e) {
		console.error(e);
		return Promise.reject(e);
	}
}

async function githubSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	const code = url.searchParams.get('code');
	if (!code) {
		return Promise.reject('Missing `code`');
	}

	try {
		const exchange = await invoke<OAuthAccessTokenExchange>('get_github_access_token', {
			code
		});
		const auth = getAuth();
		const credential = GithubAuthProvider.credential(exchange.access_token);
		return signInWithCredential(auth, credential);
	} catch (e) {
		console.error(e);
		return Promise.reject(e);
	}
}

async function twitterSignIn(payload: string, codeChallenge: string, port: number): Promise<UserCredential> {
	const url = new URL(payload);

	const maybeError = url.searchParams.get('error');
	if (maybeError) {
		return Promise.reject(maybeError ?? 'unknown error');
	}

	const code = url.searchParams.get('code');
	if (!code) {
		return Promise.reject('Missing `code`');
	}

	try {
		const exchange = await invoke<OAuthAccessTokenExchange>('get_twitter_access_token', {
			clientId: env.PUBLIC_TWITTER_CLIENT_ID,
			code,
			codeChallenge,
			port,
		})
		const credential = TwitterAuthProvider.credential(exchange.access_token,)
		return signInWithCredential(auth, credential);
	} catch (e) {
		console.error(e);
		return Promise.reject(e);
	}
}

function openGoogleSignIn(port: string): Promise<void> {
	return open(
		'https://accounts.google.com/o/oauth2/auth?' +
		'response_type=token&' +
		`client_id=${env.PUBLIC_FIREBASE_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		'scope=email%20profile&' +
		'prompt=consent'
	);
}

/*
 * https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
 */
function openGithubSignIn(port: string): Promise<void> {
	return open(
		'https://github.com/login/oauth/authorize?' +
		`client_id=${env.PUBLIC_GITHUB_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		'scope=read:user%20user:email'
	);
}

async function openTwitterSignIn(port: string): Promise<{ codeChallenge: string }> {
	const [codeChallenge, _SHA256] = await invoke<[string, string]>('generate_code_challenge');

	/**
	 * Guess what? Twitter's `code_challenge` field DIFFERS from the OAuth standard.
	 * Instead of the respected 128 character max length present for EVERY OTHER
	 * PROVIDER, Twitter decided that 100 characters is the longest this field can be.
	 * And did I mention that there are no error messages to debug this?
	 */
	const codeChallengeShortened = codeChallenge.substring(0, 100);

	// open(
	// 	'https://twitter.com/i/oauth2/authorize?' +
	// 	'response_type=code&' +
	// 	`client_id=${env.PUBLIC_TWITTER_CLIENT_ID}&` +
	// 	`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
	// 	`code_challenge=${codeChallengeShortened}&` +
	// 	'code_challenge_method=plain&' +
	// 	'state=state&' + // the only initiator is the desktop client
	// 	'scope=users.read%20offline.access'
	// );
	open(
		''
	)

	return {
		codeChallenge: codeChallengeShortened,
	}
}

export async function googleLogin(
	onSuccess: (userCredential: UserCredential) => Promise<void>,
	onRejection?: (error: any) => void
): Promise<() => Promise<void>> {
	let cancel = listen('oauth://url', async (data) => {
		try {
			const credential = await googleSignIn(data.payload as string);
			await onSuccess(credential);
		} catch (e) {
			onRejection?.(e);
		} finally {
			(await cancel)();
		}
	});

	const port: number = await invoke('plugin:oauth|start');

	openGoogleSignIn(String(port)).catch(onRejection);

	return async () => {
		await invoke('plugin:oauth|cancel', { port });
	};
}

export async function githubLogin(
	onSuccess: (userCredential: UserCredential) => Promise<void>,
	onRejection?: (error: any) => void
): Promise<() => Promise<void>> {
	let cancel = listen('oauth://url', async (data) => {
		try {
			const credential = await githubSignIn(data.payload as string);
			await onSuccess(credential);
		} catch (e) {
			onRejection?.(e);
		} finally {
			(await cancel)();
		}
	});

	const port: number = await invoke('plugin:oauth|start');

	openGithubSignIn(String(port)).catch(onRejection);

	return async () => {
		await invoke('plugin:oauth|cancel', { port });
	};
}

export async function twitterLogin(onSuccess: (userCredential: UserCredential) => Promise<void>) {
	let codeChallenge: string | undefined = undefined;

	let port: number | undefined;

	listen('oauth://url', async (data) => {
		if (codeChallenge === undefined || port === undefined) return Promise.reject("The code challenge or port was not returned");

		const credential = await twitterSignIn(data.payload as string, codeChallenge, port);
		await onSuccess(credential);
	});

	port = await invoke('plugin:oauth|start', {
		config: {
			ports: [
				7702,
				9776,
				8822,
				20166,
				20199,
				38978,
				50297,
				19684,
			]
		}
	});

	({ codeChallenge } = await openTwitterSignIn(String(port)));
}
