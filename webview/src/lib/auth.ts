import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import { getAuth, GoogleAuthProvider, signInWithCredential, type UserCredential } from 'firebase/auth';
import authPage from './authPage';
import { FIREBASE_CLIENT_ID } from './secrets';

function googleSignIn(payload: string): Promise<UserCredential> {
	const url = new URL(payload);
	const accessToken = new URLSearchParams(url.hash.substring(1)).get('access_token');
	if (!accessToken) {
		return Promise.reject('Missing `access_token`');
	};

	const auth = getAuth();

	const credential = GoogleAuthProvider.credential(null, accessToken);

	return signInWithCredential(auth, credential);
}

async function openGoogleSignIn(port: string): Promise<void> {
	console.log(port)
	return open('https://accounts.google.com/o/oauth2/auth?' +
		'response_type=token&' +
		`client_id=${FIREBASE_CLIENT_ID}&` +
		`redirect_uri=http%3A//localhost:${port}&` +
		'scope=email%20profile%20openid&' +
		'prompt=consent'
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