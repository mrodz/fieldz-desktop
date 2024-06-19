// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	interface Window {
		fieldz?: {
			deleteTeam?: (team: Team, index: number) => Promise<void>;
			editTeam?: (team: TeamExtension, index: number) => Promise<void>;
		}
	}

	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export { };
