import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import {
	FacebookAuthProvider,
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

async function twitterSignIn(payload: string, prefetch: TwitterOAuthFlowStageOne): Promise<UserCredential> {
	const url = new URL(payload);

	const maybeError = url.searchParams.get('error') ?? url.searchParams.get('denied');
	if (maybeError) {
		return Promise.reject(maybeError ?? 'unknown error');
	}

	const oauth_token = url.searchParams.get('oauth_token');
	if (!oauth_token) {
		return Promise.reject('Missing `oauth_token`');
	}

	const oauth_verifier = url.searchParams.get('oauth_verifier');
	if (!oauth_verifier) {
		return Promise.reject('Missing `oauth_verifier`');
	}

	try {
		const result = await invoke<TwitterOAuthFlowStageTwo>('finish_twitter_oauth_transaction', {
			oauthToken: oauth_token,
			oauthTokenSecret: prefetch.data?.oauth_token_secret,
			oauthVerifier: oauth_verifier,
		})

		if (!!result.error) {
			return Promise.reject(result.error);
		}

		const auth = getAuth();
		const credential = TwitterAuthProvider.credential(result.data!.oauth_token, result.data!.oauth_token_secret)
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

interface TwitterOAuthFlowStageOne {
	data?: {
		oauth_token: string;
		oauth_token_secret: string;
		authorization_url: string;
	}
	error?: string;
}

interface TwitterOAuthFlowStageTwo {
	data?: {
		oauth_token: string;
		oauth_token_secret: string;
	}
	error?: string;
}

async function openTwitterSignIn(port: string): Promise<TwitterOAuthFlowStageOne> {
	const payload = await invoke<TwitterOAuthFlowStageOne>('begin_twitter_oauth_transaction', {
		port: Number(port),
	});

	if (!!payload.error) {
		console.trace(payload.error)
		return Promise.reject(payload.error);
	}

	open(payload.data!.authorization_url);

	return payload;
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

export async function twitterLogin(
	onSuccess: (userCredential: UserCredential) => Promise<void>,
	onRejection?: (error: any) => void
): Promise<() => Promise<void>> {
	let prefetch: TwitterOAuthFlowStageOne | undefined;

	let cancel = listen('oauth://url', async (data) => {
		if (prefetch === undefined) return Promise.reject("The code challenge or port was not returned");

		try {
			const credential = await twitterSignIn(data.payload as string, prefetch);
			await onSuccess(credential);
		} catch (e) {
			onRejection?.(e);
		} finally {
			(await cancel)();
		}
	});

	const port = await invoke('plugin:oauth|start', {
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

	openTwitterSignIn(String(port)).then(data => prefetch = data).catch(onRejection);

	return async () => {
		await invoke('plugin:oauth|cancel', { port });
	};
}
