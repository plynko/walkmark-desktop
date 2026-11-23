<script lang="ts" module>
	import type { AnnotationKind, AnnotationStroke } from '$lib/types';

	export type Tool = 'select' | 'crop' | AnnotationKind;

	export const TOOLS: {
		value: Tool;
		label: string;
		icon: string;
		hint: string;
		shortcut: string;
	}[] = [
		{
			value: 'select',
			label: 'Select',
			icon: 'lucide:mouse-pointer-2',
			hint: 'Click to select an annotation, or drag to move it',
			shortcut: 'V'
		},
		{
			value: 'blur',
			label: 'Blur',
			icon: 'lucide:scan',
			hint: 'Pixelate sensitive details',
			shortcut: 'B'
		},
		{
			value: 'redact',
			label: 'Cover',
			icon: 'lucide:square',
			hint: 'Paint over a region completely',
			shortcut: 'R'
		},
		{
			value: 'highlight',
			label: 'Highlight',
			icon: 'lucide:highlighter',
			hint: 'Draw a coloured box around something',
			shortcut: 'H'
		},
		{
			value: 'crop',
			label: 'Crop',
			icon: 'lucide:crop',
			hint: 'Drag over the area to keep, then apply the crop',
			shortcut: 'C'
		},
		{
			value: 'mouse',
			label: 'Cursor',
			icon: 'lucide:mouse-pointer-2',
			hint: 'Click to place the cursor tip inside a yellow halo',
			shortcut: 'M'
		}
	];
</script>

<script lang="ts">
	import mouseMarker from '$lib/assets/mouse-marker.png';
	import type { Annotation, Rect } from '$lib/types';
	import { cn } from '$lib/utils';
	import {
		annotationClasses,
		annotationStyle,
		defaultColor,
		draftStyle
	} from './annotation-styles';

	let {
		annotations,
		tool,
		editable = true,
		color,
		stroke = 'medium',
		visibleRegion,
		cropSelection,
		onCropSelect,
		selected = $bindable<string | null>(null),
		onChange,
		class: className = ''
	}: {
		annotations: Annotation[];
		tool: Tool;
		editable?: boolean;
		/** Colour applied to newly drawn highlight / cover regions. */
		color?: string;
		stroke?: AnnotationStroke;
		visibleRegion?: Rect | null;
		cropSelection?: Rect | null;
		onCropSelect?: (rect: Rect) => void;
		selected?: string | null;
		onChange: (annotations: Annotation[]) => void;
		class?: string;
	} = $props();

	const MIN_SIZE = 0.012;

	let surfaceRef = $state<HTMLDivElement | null>(null);
	let draft = $state<Rect | null>(null);
	let origin = $state<{ x: number; y: number } | null>(null);
	let gesture = $state<{ id: number; tool: Tool; annotation?: Annotation } | null>(null);

	const visibleAnnotations = $derived.by(() => {
		const view = visibleRegion;
		if (!view) return annotations;
		return annotations.filter(({ rect }) =>
			rect.x < view.x + view.w && rect.x + rect.w > view.x &&
			rect.y < view.y + view.h && rect.y + rect.h > view.y
		);
	});

	$effect(() => {
		if (tool !== 'select') selected = null;
		cancelGesture();
	});

	function toLocal(event: PointerEvent) {
		const box = surfaceRef!.getBoundingClientRect();
		return {
			x: Math.min(1, Math.max(0, (event.clientX - box.left) / box.width)),
			y: Math.min(1, Math.max(0, (event.clientY - box.top) / box.height))
		};
	}

	function toStyle(rect: Rect) {
		return `left:${rect.x * 100}%;top:${rect.y * 100}%;width:${rect.w * 100}%;height:${rect.h * 100}%`;
	}

	function onPointerDown(event: PointerEvent) {
		if (!editable || event.button !== 0 || !event.isPrimary || gesture) return;
		if (tool === 'select') {
			const id = (event.target as Element).closest('[data-annotation-id]')?.getAttribute('data-annotation-id');
			const annotation = visibleAnnotations.find((item) => item.id === id);
			selected = annotation?.id ?? null;
			if (!annotation) return;
			gesture = { id: event.pointerId, tool, annotation };
		} else {
			gesture = { id: event.pointerId, tool };
		}
		event.preventDefault();
		surfaceRef?.setPointerCapture(event.pointerId);
		origin = toLocal(event);
		if (tool !== 'mouse' && tool !== 'select') draft = { ...origin, w: 0, h: 0 };
	}

	function onPointerMove(event: PointerEvent) {
		if (!origin || gesture?.id !== event.pointerId || gesture.tool === 'mouse') return;
		const point = toLocal(event);
		if (gesture.annotation && surfaceRef) {
			const { kind, rect } = gesture.annotation;
			const dx = point.x - origin.x;
			const dy = point.y - origin.y;
			const box = surfaceRef.getBoundingClientRect();
			if (Math.hypot(dx * box.width, dy * box.height) < 3) {
				draft = null;
				return;
			}
			const offsetX = kind === 'mouse' ? rect.w / 2 : 0;
			const offsetY = kind === 'mouse' ? rect.h / 2 : 0;
			draft = {
				...rect,
				x: Math.max(-offsetX, Math.min(1 - rect.w + offsetX, rect.x + dx)),
				y: Math.max(-offsetY, Math.min(1 - rect.h + offsetY, rect.y + dy))
			};
			return;
		}
		draft = {
			x: Math.min(origin.x, point.x),
			y: Math.min(origin.y, point.y),
			w: Math.abs(point.x - origin.x),
			h: Math.abs(point.y - origin.y)
		};
	}

	function cancelGesture() {
		gesture = null;
		origin = null;
		draft = null;
	}

	function onPointerUp(event: PointerEvent) {
		if (gesture?.id !== event.pointerId || gesture.tool !== tool) return;
		onPointerMove(event);
		let rect = draft;
		const start = origin;
		const moved = gesture.annotation;
		cancelGesture();
		if (tool === 'select') {
			if (moved && rect) {
				const movedRect = rect;
				onChange(annotations.map((annotation) =>
					annotation.id === moved.id ? { ...annotation, rect: movedRect } : annotation
				));
			}
			return;
		}
		if (tool === 'mouse' && start && surfaceRef) {
			const box = surfaceRef.getBoundingClientRect();
			const point = toLocal(event);
			if (Math.hypot((point.x - start.x) * box.width, (point.y - start.y) * box.height) > 8) return;
			const size = Math.min(44, Math.min(box.width, box.height) * 0.15);
			const w = size / box.width;
			const h = size / box.height;
			rect = { x: point.x - w / 2, y: point.y - h / 2, w, h };
		}
		if (!rect || (tool !== 'mouse' && (rect.w < MIN_SIZE || rect.h < MIN_SIZE))) return;
		if (tool === 'crop') {
			onCropSelect?.(rect);
			return;
		}

		const kind = tool;
		const next: Annotation = {
			id: crypto.randomUUID().slice(0, 12),
			kind,
			rect
		};
		const fill = color ?? defaultColor(kind);
		if (fill) next.color = fill;
		if (kind === 'highlight') next.stroke = stroke;

		onChange([...annotations, next]);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={surfaceRef}
	data-slot="annotation-surface"
	onpointerdown={onPointerDown}
	onpointermove={onPointerMove}
	onpointerup={onPointerUp}
	onpointercancel={(event) => {
		if (gesture?.id === event.pointerId) cancelGesture();
	}}
	class={cn(
		'absolute inset-0',
		!editable ? 'pointer-events-none' : tool === 'select' ? 'touch-none' : 'cursor-crosshair touch-none',
		className
	)}
>
	{#each visibleAnnotations as annotation (annotation.id)}
		{@const isSelected = selected === annotation.id}
		{@const rect = gesture?.annotation?.id === annotation.id && draft ? draft : annotation.rect}
		{#if editable && tool === 'select'}
			<button
				type="button"
				data-annotation-id={annotation.id}
				aria-pressed={isSelected}
				aria-label={annotation.kind === 'mouse' ? 'Cursor marker' : `${annotation.kind} annotation`}
				onclick={() => (selected = annotation.id)}
				style="{toStyle(rect)};{annotationStyle(annotation) ?? ''}"
				class={cn(
					annotationClasses(annotation, isSelected),
					'cursor-grab active:cursor-grabbing'
				)}
			>
				{#if annotation.kind === 'mouse'}
					<img src={mouseMarker} alt="" class="pointer-events-none absolute inset-0 size-full" />
				{/if}
			</button>
		{:else}
			<div
				aria-hidden="true"
				style="{toStyle(annotation.rect)};{annotationStyle(annotation) ?? ''}"
				class={cn(annotationClasses(annotation, isSelected), 'pointer-events-none')}
			>
				{#if annotation.kind === 'mouse'}
					<img src={mouseMarker} alt="" class="absolute inset-0 size-full" />
				{/if}
			</div>
		{/if}
	{/each}

	{#if tool === 'crop'}
		{@const region = draft ?? cropSelection}
		{#if region}
			<div
				aria-hidden="true"
				class="pointer-events-none absolute border-2 border-white shadow-[0_0_0_200vmax_#0009]"
				style={toStyle(region)}
			></div>
		{/if}
	{:else if draft && tool !== 'select'}
		<div
			class={cn(
				'pointer-events-none absolute rounded-[3px] border-2',
				tool === 'blur' && 'border-(--text)/60 bg-(--text)/15',
				tool === 'highlight' && 'border-(--text)/60',
				tool === 'redact' && 'border-(--text)/40'
			)}
			style="{toStyle(draft)};{draftStyle(tool, color) ?? ''}"
		></div>
	{/if}
</div>
