<script lang="ts">
	import Icon from '@iconify/svelte';
	import type { DocumentLabels, ResolvedLabels } from '$lib/types';
	import * as api from '$lib/api';
	import { store } from '$lib/store.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { LIMITS } from '$lib/limits';

	const language = $derived(store.settings.language.trim() || 'English');
	const saved = $derived(store.settings.labels ?? {});

	const FIELDS: { key: keyof DocumentLabels; label: string; hint: string }[] = [
		{ key: 'guide', label: 'Heading above the title', hint: 'Small line over the guide’s title.' },
		{ key: 'prerequisites', label: 'Prerequisites heading', hint: 'Heads the list of what the reader needs first.' },
		{ key: 'exported', label: 'Export date', hint: 'Comes before the date, e.g. “Exported 2026-11-23”.' },
		{ key: 'version', label: 'Version', hint: 'Comes before the version number.' },
		{ key: 'author', label: 'Author', hint: 'Comes before the author’s name.' }
	];

	let info = $state<ResolvedLabels | null>(null);
	let translating = $state(false);

	// Re-read whenever the language, or anything saved for it, changes.
	$effect(() => {
		void saved;
		const wanted = language;
		let current = true;
		void api.documentLabels(wanted).then((next) => {
			if (current) info = next;
		});
		return () => {
			current = false;
		};
	});

	/** Store only what differs from the built-in wording, so a later improvement to it still applies. */
	function save(values: DocumentLabels) {
		if (!info) return;
		const changed = Object.fromEntries(
			FIELDS.map(({ key }) => [key, values[key].trim() === info!.defaults[key] ? '' : values[key].trim()])
		) as unknown as DocumentLabels;
		const next = { ...saved };
		for (const name of Object.keys(next)) {
			if (name.trim().toLowerCase() === language.toLowerCase()) delete next[name];
		}
		if (Object.values(changed).some(Boolean)) next[language] = changed;
		void store.updateSettings({ labels: next });
	}

	function commit(key: keyof DocumentLabels, value: string) {
		if (!info || value.trim() === info.labels[key]) return;
		save({ ...info.labels, [key]: value });
	}

	async function translate() {
		if (translating) return;
		translating = true;
		try {
			save(await api.translateLabels(language));
		} catch (error) {
			store.reportError(error, 'Could not translate the wording');
		} finally {
			translating = false;
		}
	}
</script>

<div class="space-y-3">
	<div class="space-y-1">
		<Label>Wording in exports</Label>
		<p class="text-xs text-(--text)/56">
			The fixed words around your text, in {language}. They change with the language above.
			{#if info?.localized}
				Walkmark already has {language} wording; edit any of it, or leave it as it is.
			{:else if info}
				There is no built-in {language} wording. Translate it with AI or type your own.
			{/if}
		</p>
	</div>

	{#if info}
		<div class="flex items-center gap-2">
			{#if !info.localized}
				<Button
					size="sm"
					variant="ghost"
					disabled={translating || !store.canWrite}
					data-tooltip={store.canWrite ? '' : 'Choose a model in the Model tab first'}
					onclick={() => void translate()}
				>
					<Icon icon="lucide:sparkles" class="size-3.5" aria-hidden="true" />
					{translating ? 'Translating…' : 'Translate with AI'}
				</Button>
			{/if}
			{#if info.customized}
				<Button
					size="sm"
					variant="ghost"
					onclick={() => {
						const next = { ...saved };
						for (const name of Object.keys(next)) {
							if (name.trim().toLowerCase() === language.toLowerCase()) delete next[name];
						}
						void store.updateSettings({ labels: next });
					}}
				>
					{info.localized ? 'Back to the built-in wording' : 'Clear'}
				</Button>
			{/if}
		</div>

		<div class="space-y-2.5">
			{#each FIELDS as field (field.key)}
				<div class="space-y-1">
					<Label class="text-xs font-medium text-(--text)/72">{field.label}</Label>
					<Input
						value={info.labels[field.key]}
						maxlength={LIMITS.documentLabel}
						placeholder={info.defaults[field.key]}
						aria-label="{field.label} in {language}"
						data-tooltip={field.hint}
						onchange={(e) => commit(field.key, (e.currentTarget as HTMLInputElement).value)}
						onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
					/>
				</div>
			{/each}
		</div>
	{/if}
</div>
