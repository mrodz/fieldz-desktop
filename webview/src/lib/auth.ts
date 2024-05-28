import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import { getAuth, GoogleAuthProvider, signInWithCredential, type UserCredential } from 'firebase/auth';
import authPage from './authPage';
import { FIREBASE_CLIENT_ID } from './secrets';
import type { GoogleOAuthAccessTokenExchange } from '$lib';

let codeVerifier: string | undefined;

async function googleSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	console.log(url);
	const code = url.searchParams.get('code');
	if (!code) {
		return Promise.reject('Missing `code`');
	};

	if (!codeVerifier) {
		return Promise.reject('Missing `codeChallenge`');
	}

	try {
		const accessToken = await invoke<GoogleOAuthAccessTokenExchange>('get_access_token', { code, clientId: FIREBASE_CLIENT_ID, codeChallenge: codeVerifier });
		console.log(accessToken);

		const auth = getAuth();

		const credential = GoogleAuthProvider.credential(null, accessToken.access_token);

		return signInWithCredential(auth, credential);
	} catch (e) {
		console.error(e);
		return Promise.reject(e);
	}
}

async function openGoogleSignIn(port: string): Promise<void> {
	const [plain, codeChallenge] = await invoke<[string, string]>('generate_code_challenge');
	console.log(plain, codeChallenge);
	codeVerifier = codeChallenge;
	return open('https://accounts.google.com/o/oauth2/auth?' +
		'response_type=code&' +
		`client_id=${FIREBASE_CLIENT_ID}&` +
		`redirect_uri=http%3A//127.0.0.1%3A${port}&` +
		'scope=email%20profile%20openid&' +
		'prompt=consent&' +
		`code_challenge=${codeChallenge}&` +
		'code_challenge_method=S256'
	);
}

export async function login(onSuccess: (userCredential: UserCredential) => Promise<void>) {
	listen('oauth://url', async (data) => {
		console.log(data);
		const credential = await googleSignIn(data.payload as string);
		await onSuccess(credential);
	});

	listen('oauth://invalid-url', (data) => {
		console.error(data);
	})

	const port: number = await invoke('plugin:oauth|start', {
		config: {
			response: authPage,

		},
	});

	await openGoogleSignIn(String(port));
}

export async function signOut() {
	await getAuth().signOut();
}