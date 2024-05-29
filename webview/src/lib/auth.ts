import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import { getAuth, GithubAuthProvider, GoogleAuthProvider, signInWithCredential, type UserCredential } from 'firebase/auth';
import { FIREBASE_CLIENT_ID, GITHUB_CLIENT_ID } from './secrets';
import type { GithubOAuthAccessTokenExchange } from '$lib';

async function googleSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	console.log(url);
	const accessToken = new URLSearchParams(url.hash.substring(1)).get('access_token');
	if (!accessToken) {
		return Promise.reject('Missing `access_token`');
	};

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

async function openGoogleSignIn(port: string): Promise<void> {
	return open('https://accounts.google.com/o/oauth2/auth?' +
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
async function openGithubSignIn(port: string): Promise<void> {
	return open('https://github.com/login/oauth/authorize?' +
		`client_id=${GITHUB_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		'scope=read:user%20user:email'
	)
}

export async function googleLogin(onSuccess: (userCredential: UserCredential) => Promise<void>) {
	listen('oauth://url', async (data) => {
		console.log(data);
		const credential = await googleSignIn(data.payload as string);
		await onSuccess(credential);
	});

	const port: number = await invoke('plugin:oauth|start');

	await openGoogleSignIn(String(port));
}

export async function githubLogin(onSuccess: (userCredential: UserCredential) => Promise<void>) {
	listen('oauth://url', async (data) => {
		console.log(data);
		const credential = await githubSignIn(data.payload as string);
		await onSuccess(credential);
	});

	const port: number = await invoke('plugin:oauth|start');

	await openGithubSignIn(String(port));
}