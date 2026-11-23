<script lang="ts">
	import Icon from '@iconify/svelte';
	import { store } from '$lib/store.svelte';
	import { Button } from '$lib/components/ui/button';
	import type { ButtonVariant } from '$lib/components/ui/button/button.svelte';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';

	let { variant = 'ghost', class: className }: { variant?: ButtonVariant; class?: string } = $props();
</script>

<DropdownMenu>
	<DropdownMenuTrigger>
		{#snippet child({ props })}
			<Button
				{...props}
				size="sm"
				{variant}
				class={className}
				disabled={store.stepImportPending || store.stepDeletionPending}
			>
				<Icon icon="lucide:plus" class="size-3.5" aria-hidden="true" />
				Add step
				<Icon icon="lucide:chevron-down" class="size-3" aria-hidden="true" />
			</Button>
		{/snippet}
	</DropdownMenuTrigger>
	<DropdownMenuContent>
		<DropdownMenuItem onSelect={() => store.setDialog('capture-step')}>
			<Icon icon="lucide:camera" />
			Capture screenshot
		</DropdownMenuItem>
		<DropdownMenuItem onSelect={() => void store.importStep()}>
			<Icon icon="lucide:images" />
			Import image
		</DropdownMenuItem>
		<DropdownMenuItem onSelect={() => void store.pasteStep()}>
			<Icon icon="lucide:copy" />
			Paste from clipboard
		</DropdownMenuItem>
	</DropdownMenuContent>
</DropdownMenu>
