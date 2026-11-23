<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { tick } from 'svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { previewMarkdown } from '$lib/api';
	import { autosize } from '$lib/autosize';
	import { store } from '$lib/store.svelte';

	let {
		value = $bindable(''),
		label,
		class: className,
		...inputProps
	}: Omit<HTMLTextareaAttributes, 'value'> & { value?: string; label: string } = $props();

	let input: HTMLTextAreaElement;
	let preview = $state(false);
	let rendered = $state<string | null>(null);
	let failed = $state(false);

	$effect(() => {
		if (!preview) return;
		const markdown = value;
		let current = true;
		rendered = null;
		failed = false;
		void previewMarkdown(markdown)
			.then((html) => { if (current) rendered = html; })
			.catch(() => { if (current) failed = true; });
		return () => { current = false; };
	});

	async function togglePreview() {
		if (preview) {
			preview = false;
			await tick();
			input?.focus();
		} else {
			input?.blur();
			preview = true;
		}
	}

	function openLinks(node: HTMLElement) {
		function follow(event: MouseEvent) {
			if (!(event.target instanceof Element)) return;
			const link = event.target.closest<HTMLAnchorElement>('a[href]');
			if (!link) return;
			event.preventDefault();
			if (event.type === 'auxclick' && event.button !== 1) return;
			void openUrl(link.href).catch((error) => store.reportError(error, 'Could not open the link'));
		}
		node.addEventListener('click', follow);
		node.addEventListener('auxclick', follow);
		return {
			destroy() {
				node.removeEventListener('click', follow);
				node.removeEventListener('auxclick', follow);
			}
		};
	}
</script>

<textarea
	bind:this={input}
	bind:value
	use:autosize={value}
	{...inputProps}
	rows={1}
	class={className}
	aria-label={label}
	hidden={preview}
></textarea>

{#if preview}
	<div
		class="markdown-preview mt-2 rounded-xl bg-(--text)/5 px-3 py-2 text-[13.5px] leading-relaxed text-(--text)/80"
		role="region"
		aria-label="{label} preview"
		use:openLinks
	>
		{#if rendered !== null}
			{@html rendered}
		{:else if failed}
			<p>Preview unavailable.</p>
		{:else}
			<p>Loading preview…</p>
		{/if}
	</div>
{/if}

<button
	type="button"
	class="block rounded px-1.5 py-0.5 text-[11px] font-medium text-(--text)/60 hover:bg-(--text)/5 hover:text-(--text) disabled:opacity-50"
	data-tooltip="Markdown supports bold, italics, lists, links and code"
	aria-label={preview ? `Edit ${label.toLowerCase()}` : `Preview ${label.toLowerCase()} as Markdown`}
	disabled={!preview && !value.trim()}
	onclick={() => void togglePreview()}
>
	{preview ? 'Edit Markdown' : 'Preview Markdown'}
</button>

<style>
	.markdown-preview { overflow-wrap: anywhere; }
	.markdown-preview :global(* + :where(p, ul, ol, blockquote, pre, table, h1, h2, h3, h4, h5, h6)) {
		margin-top: .55em;
	}
	.markdown-preview :global(:where(h1, h2, h3, h4, h5, h6)) { font-weight: 650; }
	.markdown-preview :global(ul) { list-style: disc; padding-inline-start: 1.4em; }
	.markdown-preview :global(ol) { list-style: decimal; padding-inline-start: 1.4em; }
	.markdown-preview :global(a) { text-decoration: underline; }
	.markdown-preview :global(blockquote) { border-inline-start: 2px solid currentColor; padding-inline-start: .75em; }
	.markdown-preview :global(pre) { white-space: pre-wrap; }
	.markdown-preview :global(code) { background: var(--bg); border-radius: 4px; padding: .08em .25em; }
	.markdown-preview :global(th), .markdown-preview :global(td) {
		border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
		padding: .15em .4em;
	}
</style>
