<script lang="ts">
	import { onMount, tick } from 'svelte';

	let text = $state('');
	let bubble = $state<HTMLElement>();
	let position = $state<{ left: number; top: number }>();
	let hovered: HTMLElement | null = null;
	let focused: HTMLElement | null = null;
	let target: HTMLElement | null = null;
	let timer: ReturnType<typeof setTimeout> | undefined;

	function trigger(node: EventTarget | null) {
		return node instanceof Element ? node.closest<HTMLElement>('[data-tooltip]') : null;
	}

	function positionBubble() {
		if (!target?.isConnected || !bubble) return;
		const anchor = target.getBoundingClientRect();
		const { width, height } = bubble.getBoundingClientRect();
		const left = Math.max(8, Math.min(innerWidth - width - 8, anchor.left + (anchor.width - width) / 2));
		const above = anchor.top - height - 7;
		const top = above >= 4 ? above : Math.min(innerHeight - height - 4, anchor.bottom + 7);
		position = { left, top: Math.max(4, top) };
	}

	function show(next: HTMLElement | null, delay: number) {
		if (next === target) return;
		const label = next?.dataset.tooltip || next?.getAttribute('aria-label');
		clearTimeout(timer);
		target = next;
		text = '';
		position = undefined;
		if (!next || !label) return;
		timer = setTimeout(async () => {
			if (!next.isConnected || next !== target) return;
			text = label;
			await tick();
			if (next === target) positionBubble();
		}, delay);
	}

	onMount(() => {
		const handleOver = (event: PointerEvent) => {
			if (event.pointerType === 'touch') return;
			hovered = trigger(event.target);
			show(hovered ?? focused, 350);
		};
		const handleOut = (event: PointerEvent) => {
			if (event.pointerType === 'touch') return;
			hovered = trigger(event.relatedTarget);
			show(hovered ?? focused, hovered ? 350 : 0);
		};
		const handleFocus = (event: FocusEvent) => {
			focused = event.target instanceof Element && event.target.matches(':focus-visible')
				? trigger(event.target)
				: null;
			if (focused) show(focused, 0);
		};
		const handleBlur = () => {
			focused = null;
			show(hovered, 350);
		};
		const dismiss = () => {
			hovered = null;
			focused = null;
			show(null, 0);
		};
		const handleKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape') dismiss();
		};
		const handleScroll = () => {
			if (!target) return;
			const rect = target.getBoundingClientRect();
			if (!target.isConnected || rect.bottom <= 0 || rect.top >= innerHeight) dismiss();
			else positionBubble();
		};

		window.addEventListener('pointerover', handleOver, true);
		window.addEventListener('pointerout', handleOut, true);
		window.addEventListener('pointerdown', dismiss, true);
		window.addEventListener('focusin', handleFocus, true);
		window.addEventListener('focusout', handleBlur, true);
		window.addEventListener('keydown', handleKey, true);
		window.addEventListener('scroll', handleScroll, true);
		window.addEventListener('resize', dismiss);
		window.addEventListener('blur', dismiss);
		return () => {
			dismiss();
			window.removeEventListener('pointerover', handleOver, true);
			window.removeEventListener('pointerout', handleOut, true);
			window.removeEventListener('pointerdown', dismiss, true);
			window.removeEventListener('focusin', handleFocus, true);
			window.removeEventListener('focusout', handleBlur, true);
			window.removeEventListener('keydown', handleKey, true);
			window.removeEventListener('scroll', handleScroll, true);
			window.removeEventListener('resize', dismiss);
			window.removeEventListener('blur', dismiss);
		};
	});
</script>

{#if text}
	<div
		bind:this={bubble}
		role="tooltip"
		class="pointer-events-none fixed z-[100] w-max max-w-[min(18rem,calc(100vw-16px))] rounded-lg bg-(--text) px-2 py-1 text-center text-[11px] font-medium text-(--bg) shadow-md"
		style:left="{position?.left ?? 0}px"
		style:top="{position?.top ?? 0}px"
		style:visibility={position ? 'visible' : 'hidden'}
	>
		{text}
	</div>
{/if}
