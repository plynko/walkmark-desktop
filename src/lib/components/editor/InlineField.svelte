<script lang="ts">
	import { cn } from '$lib/utils';

	let {
		value,
		label,
		placeholder = '',
		maxlength,
		class: className,
		onCommit
	}: {
		value: string;
		/** Accessible name — what this field is, not what it contains. */
		label: string;
		/** Shown while empty. For labels this is the wording exports use by default. */
		placeholder?: string;
		maxlength?: number;
		class?: string;
		onCommit: (value: string) => void;
	} = $props();

	let draft = $state('');
	let editing = $state(false);

	$effect(() => {
		if (!editing) draft = value;
	});

	function finish() {
		editing = false;
		const next = draft.trim();
		if (next !== value) onCommit(next);
	}
</script>

<!-- The hidden span sizes the field to its text, in whatever font it is styled with. -->
<span class="inline-grid max-w-full">
	<span
		aria-hidden="true"
		class={cn('invisible py-0.5 pr-2.5 pl-1 whitespace-pre [grid-area:1/1]', className)}
	>{draft || placeholder || ' '}</span>
	<input
		bind:value={draft}
		{placeholder}
		{maxlength}
		aria-label={label}
		onfocus={() => (editing = true)}
		onblur={finish}
		onkeydown={(e) => {
			if (e.key === 'Enter') e.currentTarget.blur();
			if (e.key === 'Escape') {
				draft = value;
				e.currentTarget.blur();
			}
		}}
		class={cn(
			'w-0 min-w-full rounded-lg bg-transparent px-1 py-0.5 outline-none [grid-area:1/1] placeholder:text-(--text)/32 hover:bg-(--text)/5 focus:bg-(--text)/8 focus:ring-1 focus:ring-(--text)/24',
			className
		)}
	/>
</span>
