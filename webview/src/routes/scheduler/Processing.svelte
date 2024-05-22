<script lang="ts">
	import { invoke } from "@tauri-apps/api";
	import { onMount } from "svelte";

    onMount(async () => {
        const backendHealth = await invoke('health_probe');
    })
</script>

<div class="card w-modal p-5">
    Hello
</div>