<script lang="ts">
	import * as api from '$lib/api';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { cn } from '$lib/utils';

	let {
		projectId,
		frame,
		class: className = '',
		alt = '',
		onload
	}: {
		projectId: string;
		frame: string;
		class?: string;
		alt?: string;
		onload?: (event: Event) => void;
	} = $props();

	let url = $state<string | null>(null);
	let image = $state<HTMLImageElement | null>(null);
	// Remember which address loaded or failed instead of keeping a flag that has
	// to be reset. The address stays the same when the project object is
	// replaced (after any edit), and the browser does not load a picture it
	// already has a second time, so a reset flag would stay false and the
	// thumbnail would stay invisible.
	let loadedUrl = $state<string | null>(null);
	let failedUrl = $state<string | null>(null);

	$effect(() => {
		if (!frame) {
			url = null;
			return;
		}
		let active = true;
		void api
			.frameUrl(projectId, frame)
			.then((resolved) => {
				if (active) url = resolved;
			})
			.catch(() => {
				if (active) url = null;
			});
		return () => {
			active = false;
		};
	});

	// A cached picture can be complete before our load handler is attached.
	$effect(() => {
		if (image && url && image.complete && image.naturalWidth > 0) loadedUrl = url;
	});

	const visible = $derived(url !== null && loadedUrl === url);
	const failed = $derived(url !== null && failedUrl === url);
</script>

{#if !url || failed}
	<Skeleton class={className} />
{:else}
	<img
		bind:this={image}
		src={url}
		{alt}
		draggable={false}
		onload={(event) => {
			loadedUrl = (event.currentTarget as HTMLImageElement).getAttribute('src');
			onload?.(event);
		}}
		onerror={(event) => {
			failedUrl = (event.currentTarget as HTMLImageElement).getAttribute('src');
		}}
		class={cn('transition-opacity duration-200', visible ? 'opacity-100' : 'opacity-0', className)}
	/>
{/if}
