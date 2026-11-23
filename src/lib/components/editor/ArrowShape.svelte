<script lang="ts">
	import type { ArrowDirection, Rect } from '$lib/types';

	let { rect, direction, surfaceWidth, surfaceHeight, color }: {
		rect: Rect;
		direction?: ArrowDirection | null;
		surfaceWidth: number;
		surfaceHeight: number;
		color: string;
	} = $props();

	const width = $derived(rect.w * surfaceWidth);
	const height = $derived(rect.h * surfaceHeight);
	const startX = $derived(direction?.reverseX ? width : 0);
	const startY = $derived(direction?.reverseY ? height : 0);
	const endX = $derived(width - startX);
	const endY = $derived(height - startY);
	const distance = $derived(Math.hypot(width, height));
	const stroke = $derived(Math.max(1, surfaceWidth * 0.0035));
	const head = $derived.by(() => {
		if (distance === 0) return '';
		const x = (endX - startX) / distance;
		const y = (endY - startY) / distance;
		const length = Math.min(distance * 0.38, stroke * 4.5);
		const spread = length * 0.48;
		const baseX = endX - x * length;
		const baseY = endY - y * length;
		return `M ${baseX - y * spread} ${baseY + x * spread} L ${endX} ${endY} L ${baseX + y * spread} ${baseY - x * spread}`;
	});
</script>

{#if distance > 0}
	<svg
		aria-hidden="true"
		class="pointer-events-none absolute top-[7px] left-[7px] h-px w-px overflow-visible"
		fill="none"
		stroke={color}
		stroke-width={stroke}
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="M {startX} {startY} L {endX} {endY} {head}" />
	</svg>
{/if}
