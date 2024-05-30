import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import {
	getAuth,
	GithubAuthProvider,
	GoogleAuthProvider,
	signInWithCredential,
	type UserCredential
} from 'firebase/auth';
import { FIREBASE_CLIENT_ID, GITHUB_CLIENT_ID, TWITTER_CLIENT_ID } from './secrets';
import type { GithubOAuthAccessTokenExchange } from '$lib';

async function googleSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	console.log(url);
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
		const exchange = await invoke<GithubOAuthAccessTokenExchange>('get_github_access_token', {
			code,
			clientId: GITHUB_CLIENT_ID
		});
		const auth = getAuth();
		const credential = GithubAuthProvider.credential(exchange.access_token);
		return signInWithCredential(auth, credential);
	} catch (e) {
		console.error(e);
		return Promise.reject(e);
	}
}

async function twitterSignIn(payload: string): Promise<UserCredential> {
	console.log(payload);

	return void 0 as any;
}

function openGoogleSignIn(port: string): Promise<void> {
	return open(
		'https://accounts.google.com/o/oauth2/auth?' +
		'response_type=token&' +
		`client_id=${FIREBASE_CLIENT_ID}&` +
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
		`client_id=${GITHUB_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		'scope=read:user%20user:email'
	);
}

async function openTwitterSignIn(port: string): Promise<void> {
	const [codeChallenge, _SHA256] = await invoke<[string, string]>('generate_code_challenge');

	/**
	 * Guess what? Twitter's `code_challenge` field DIFFERS from the OAuth standard.
	 * Instead of the respected 128 characted max length present for EVERY OTHER
	 * PROVIDER, Twitter decided that 100 characters is the longest this field can be.
	 * And did I mention that there are no error messages to debug this?
	 */
	const codeChallengeShortened = codeChallenge.substring(0, 100);

	return open(
		'https://twitter.com/i/oauth2/authorize?' +
		'response_type=code&' +
		`client_id=${TWITTER_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		`code_challenge=${codeChallengeShortened}&` +
		'code_challenge_method=plain&' +
		'state=state&' + // the only initiator is the desktop client
		'scope=users.read'
	);
}

export async function googleLogin(onSuccess: (userCredential: UserCredential) => Promise<void>, onRejection?: (error: any) => void) {
	listen('oauth://url', async (data) => {
		console.log(data);
		try {
			const credential = await googleSignIn(data.payload as string);
			await onSuccess(credential);
		} catch (e) {
			onRejection?.(e);
		}
	});

	const port: number = await invoke('plugin:oauth|start');

	await openGoogleSignIn(String(port));
}

export async function githubLogin(onSuccess: (userCredential: UserCredential) => Promise<void>, onRejection?: (error: any) => void) {
	listen('oauth://url', async (data) => {
		console.log(data);
		try {
			const credential = await githubSignIn(data.payload as string);
			await onSuccess(credential);
		} catch (e) {
			onRejection?.(e);
		}
	});

	const port: number = await invoke('plugin:oauth|start');

	await openGithubSignIn(String(port));
}

export async function twitterLogin(onSuccess: (userCredential: UserCredential) => Promise<void>) {
	listen('oauth://url', async (data) => {
		console.log(data);
		const credential = await twitterSignIn(data.payload as string);
		await onSuccess(credential);
	});

	const port: number = await invoke('plugin:oauth|start', {
		config: {
			ports: [4321]
		}
	});

	alert(port);

	await openTwitterSignIn(String(port));
}
