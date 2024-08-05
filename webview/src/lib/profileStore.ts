import { writable } from 'svelte/store';

const profileStore = writable<{
	name: Promise<string | null>;
}>({
	name: new Promise(() => {})
});

export default {
	subscribe: profileStore.subscribe,
	set: profileStore.set
};
