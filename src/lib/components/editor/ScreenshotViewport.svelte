<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Rect } from '$lib/types';
	import { cn } from '$lib/utils';

	let { src, alt, crop, maxHeight, children, class: className = '' }: {
		src: string;
		alt: string;
		crop?: Rect | null;
		maxHeight?: string;
		children?: Snippet;
		class?: string;
	} = $props();

	let image = $state<HTMLImageElement>();
	let naturalWidth = $state(0);
	let naturalHeight = $state(0);
	const view = $derived(crop ?? { x: 0, y: 0, w: 1, h: 1 });
	const ratio = $derived(naturalWidth * view.w / (naturalHeight * view.h));
</script>

<!-- The inner surface keeps the coordinate system of the original frame. -->
<div
	data-slot="screenshot-viewport"
	class={cn('relative w-full overflow-hidden supports-[overflow:clip]:overflow-clip', className)}
	style:aspect-ratio={ratio > 0 ? ratio : undefined}
	style:width={ratio > 0 && maxHeight ? `min(100%, calc(${maxHeight} * ${ratio}))` : undefined}
>
	<div
		class={ratio > 0 ? 'absolute' : 'relative'}
		style:width="{100 / view.w}%"
		style:height="{100 / view.h}%"
		style:left="{-100 * view.x / view.w}%"
		style:top="{-100 * view.y / view.h}%"
	>
		<img
			bind:this={image}
			{src}
			{alt}
			draggable={false}
			bind:naturalWidth
			bind:naturalHeight
			class="block w-full"
		/>
		{#if image && ratio > 0}
			{@render children?.()}
		{/if}
	</div>
</div>
