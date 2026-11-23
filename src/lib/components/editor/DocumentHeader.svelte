<script lang="ts">
	import Icon from '@iconify/svelte';
	import { pluralize, relativeTime } from '$lib/format';
	import { productLabel } from '$lib/products';
	import { languageShortLabel } from '$lib/languages';
	import { store } from '$lib/store.svelte';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import { LIMITS } from '$lib/limits';
	import { autosize } from '$lib/autosize';
	import MarkdownField from './MarkdownField.svelte';
	import InlineField from './InlineField.svelte';
	import type { DocumentLabels } from '$lib/types';

	// What exports print when a label is left empty. Mirrors `DocumentLabels` in Rust.
	const DEFAULT_LABELS: DocumentLabels = {
		guide: 'Step-by-step guide',
		prerequisites: 'Before you start',
		exported: 'Exported',
		version: 'Version',
		author: 'Author'
	};

	const project = $derived(store.project!);
	const recording = $derived(store.recording);

	const included = $derived(project.steps.filter((s) => s.include).length);
	const written = $derived(
		project.steps.filter((s) => s.include && s.status === 'ready').length
	);
	const product = $derived(productLabel(store.settings, project.productId));
	const language = $derived(languageShortLabel(project.language || store.settings.language));

	// Title field
	let titleDraft = $state('');
	let titleEditing = $state(false);

	// Summary field
	let summaryDraft = $state('');
	let summaryEditing = $state(false);

	// Prerequisites
	let addingPrereq = $state(false);
	let prereqDraft = $state('');
	let prereqInputRef = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (!titleEditing) titleDraft = project.title;
	});
	$effect(() => {
		if (!summaryEditing) summaryDraft = project.summary;
	});
	$effect(() => {
		if (addingPrereq) prereqInputRef?.focus();
	});

	function patchMeta(meta: Parameters<typeof store.patchMeta>[0]) {
		void store.patchMeta(meta);
	}

	function patchLabel(key: keyof DocumentLabels, value: string) {
		patchMeta({ labels: { ...project.labels, [key]: value } });
	}

	// The real date is stamped at export time; this is only a preview of it.
	const today = new Date().toLocaleDateString('sv-SE');

	function commitPrereq() {
		const value = prereqDraft.trim();
		if (value && project.prerequisites.length < LIMITS.prerequisitesMax) {
			patchMeta({ prerequisites: [...project.prerequisites, value] });
		}
		prereqDraft = '';
		addingPrereq = false;
	}
</script>

<header class="border-b border-(--text)/8 pb-7">
	<div class="mb-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-(--text)/40">
		<span>{product}</span>
		<span aria-hidden="true">·</span>
		{#if project.sourceLabel}
			<span class="truncate">Recorded from {project.sourceLabel}</span>
			<span aria-hidden="true">·</span>
		{/if}
		<span>{relativeTime(project.updatedAt)}</span>
		<span aria-hidden="true">·</span>
		<span>{language}</span>
		{#if included > 0 && !recording}
			<span aria-hidden="true">·</span>
			<span class={cn(written < included && 'text-amber-600 dark:text-amber-400')}>
				{written === included
					? 'All steps written'
					: `${included - written} of ${pluralize(included, 'step')} unwritten`}
			</span>
		{/if}
	</div>

	<InlineField
		value={project.labels.guide}
		label="Guide label"
		placeholder={DEFAULT_LABELS.guide}
		maxlength={LIMITS.documentLabel}
		onCommit={(value) => patchLabel('guide', value)}
		class="-mx-1 mb-1 text-[11.5px] font-semibold tracking-wider text-(--text)/56 uppercase"
	/>

	<textarea
		use:autosize={titleDraft}
		rows={1}
		bind:value={titleDraft}
		placeholder="Untitled guide"
		maxlength={LIMITS.documentTitle}
		onfocus={() => (titleEditing = true)}
		onblur={() => {
			titleEditing = false;
			if (titleDraft.trim() !== project.title) patchMeta({ title: titleDraft.trim() });
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				titleDraft = project.title;
				e.currentTarget.blur();
			}
		}}
		class="-mx-1.5 w-full resize-none overflow-hidden rounded-xl bg-transparent px-1.5 py-0.5 text-[27px] font-semibold leading-[1.2] tracking-[-0.02em] outline-none placeholder:text-(--text)/40 hover:bg-(--text)/5 focus:bg-(--text)/8 focus:ring-1 focus:ring-(--text)/24"
	></textarea>

	<div class="mt-2 -mx-1 flex flex-wrap items-center gap-x-1 gap-y-1 text-[12.5px] text-(--text)/56">
		<InlineField
			value={project.labels.exported}
			label="Export date label"
			placeholder={DEFAULT_LABELS.exported}
			maxlength={LIMITS.documentLabel}
			onCommit={(value) => patchLabel('exported', value)}
		/>
		<span
			class="-ml-1 tabular-nums"
			data-tooltip="Filled in automatically on the day you export"
			title="Filled in automatically on the day you export">{today}</span>
		<span aria-hidden="true">·</span>
		<InlineField
			value={project.labels.version}
			label="Version label"
			placeholder={DEFAULT_LABELS.version}
			maxlength={LIMITS.documentLabel}
			onCommit={(value) => patchLabel('version', value)}
		/>
		<InlineField
			value={project.version}
			label="Version"
			placeholder="1.0"
			maxlength={LIMITS.documentVersion}
			onCommit={(value) => patchMeta({ version: value })}
			class="-ml-1 text-(--text)/72"
		/>
		<span aria-hidden="true">·</span>
		<InlineField
			value={project.labels.author}
			label="Author label"
			placeholder={DEFAULT_LABELS.author}
			maxlength={LIMITS.documentLabel}
			onCommit={(value) => patchLabel('author', value)}
		/>
		<InlineField
			value={project.author}
			label="Author"
			placeholder="Your name"
			maxlength={LIMITS.documentAuthor}
			onCommit={(value) => patchMeta({ author: value })}
			class="-ml-1 text-(--text)/72"
		/>
		<Button
			size="sm"
			variant="ghost"
			class="ml-auto"
			disabled={store.translatingLabels}
			data-tooltip="Translate the labels above and “Before you start” with AI, into the guide's language"
			onclick={() => void store.translateLabels()}
		>
			<Icon icon="lucide:sparkles" class="size-3.5" aria-hidden="true" />
			{store.translatingLabels ? 'Translating…' : 'Translate labels'}
		</Button>
	</div>

	<MarkdownField
		label="Guide summary"
		bind:value={summaryDraft}
		placeholder="Add a sentence about what this guide covers and when to use it."
		maxlength={LIMITS.documentSummary}
		onfocus={() => (summaryEditing = true)}
		onblur={() => {
			summaryEditing = false;
			if (summaryDraft.trim() !== project.summary) patchMeta({ summary: summaryDraft.trim() });
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				summaryDraft = project.summary;
				e.currentTarget.blur();
			}
		}}
		class="mt-2.5 -mx-1.5 w-full resize-none overflow-hidden rounded-xl bg-transparent px-1.5 py-0.5 text-[14.5px] leading-relaxed text-(--text)/72 outline-none placeholder:text-(--text)/40 hover:bg-(--text)/5 focus:bg-(--text)/8 focus:ring-1 focus:ring-(--text)/24"
	/>

	{#if project.prerequisites.length === 0 && !addingPrereq}
		<button
			type="button"
			onclick={() => (addingPrereq = true)}
			class="-ml-1.5 mt-4 inline-flex items-center gap-1.5 rounded-xl px-1.5 py-1 text-[12.5px] text-(--text)/40 transition-colors hover:bg-(--text)/5 hover:text-(--text)/72"
		>
			<Icon icon="lucide:list-checks" class="size-3.5" aria-hidden="true" />
			Add what the reader needs before starting
		</button>
	{:else}
		<div class="mt-5 rounded-3xl bg-(--text)/5 p-3.5">
			<InlineField
				value={project.labels.prerequisites}
				label="Prerequisites heading"
				placeholder={DEFAULT_LABELS.prerequisites}
				maxlength={LIMITS.documentLabel}
				onCommit={(value) => patchLabel('prerequisites', value)}
				class="-ml-1 mb-1.5 text-[11.5px] font-semibold tracking-wider text-(--text)/56 uppercase"
			/>
			<ul class="flex flex-col gap-1">
				{#each project.prerequisites as item, index (item + index)}
					<li class="group flex items-center gap-2">
						<span class="size-1 flex-none rounded-full bg-(--text)/40" aria-hidden="true"></span>
						<span class="min-w-0 flex-1 truncate text-[13px] text-(--text)/72">{item}</span>
						<button
							type="button"
							aria-label="Remove “{item}”" data-tooltip=""
							onclick={() =>
								patchMeta({
									prerequisites: project.prerequisites.filter((_, i) => i !== index)
								})}
							class="rounded p-0.5 text-(--text)/40 opacity-0 transition-opacity hover:text-red-500 group-hover:opacity-100 focus-visible:opacity-100"
						>
							<Icon icon="lucide:x" class="size-3" aria-hidden="true" />
						</button>
					</li>
				{/each}
			</ul>

			{#if addingPrereq}
				<input
					bind:this={prereqInputRef}
					bind:value={prereqDraft}
					maxlength={LIMITS.prerequisite}
					placeholder="e.g. An admin account"
					onblur={commitPrereq}
					onkeydown={(e) => {
						if (e.key === 'Enter') commitPrereq();
						if (e.key === 'Escape') {
							prereqDraft = '';
							addingPrereq = false;
						}
					}}
					class="mt-2 w-full rounded-3xl border border-(--text)/12 bg-(--bg-elevated) px-3 py-1.5 text-[13px] outline-none"
				/>
			{:else}
				<Button
					size="sm"
					variant="ghost"
					class="-ml-2 mt-1.5"
					disabled={project.prerequisites.length >= LIMITS.prerequisitesMax}
					onclick={() => (addingPrereq = true)}
				>
					<Icon icon="lucide:plus" class="size-3.5" />
					Add item
				</Button>
			{/if}
		</div>
	{/if}
</header>
