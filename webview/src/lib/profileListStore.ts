import { writable } from 'svelte/store';

const profileList = writable<Promise<[string, { size?: number }][]>>(Promise.resolve([]));

export default {
	set: profileList.set,
	subscribe: profileList.subscribe
};
