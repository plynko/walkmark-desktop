<script lang="ts">
	import { onMount } from "svelte";
	import Icon from "@iconify/svelte";
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { store } from "$lib/store.svelte";
	import { pluralize, relativeTime } from "$lib/format";
	import { LIMITS } from "$lib/limits";
	import { Button } from "$lib/components/ui/button";
	import { Badge } from "$lib/components/ui/badge";
	import { Input } from "$lib/components/ui/input";
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle,
	} from "$lib/components/ui/dialog";
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuTrigger,
	} from "$lib/components/ui/dropdown-menu";
	import Logo from "$lib/components/Logo.svelte";
	import LibrarySkeleton from "$lib/components/skeletons/LibrarySkeleton.svelte";
	import type { ProjectSummary } from "$lib/types";

	const PAGE_SIZE = 12;

	let deleteTarget = $state<ProjectSummary | null>(null);
	let deleting = $state(false);
	let query = $state("");
	let page = $state(1);

	type View = "cards" | "details";
	const VIEW_KEY = "walkmark.libraryView";

	function savedView(): View {
		try {
			return localStorage.getItem(VIEW_KEY) === "details" ? "details" : "cards";
		} catch {
			return "cards";
		}
	}

	let view = $state<View>(savedView());

	function setView(next: View) {
		view = next;
		try {
			localStorage.setItem(VIEW_KEY, next);
		} catch {
			// The choice just won't be remembered.
		}
	}

	function writtenOn(iso: string): string {
		const date = new Date(iso);
		if (Number.isNaN(date.getTime())) return "";
		return date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
	}

	const filtered = $derived(
		store.projects.filter((project) => {
			const needle = query.trim().toLowerCase();
			if (!needle) return true;
			return project.title.toLowerCase().includes(needle);
		}),
	);

	const totalPages = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));

	const pageItems = $derived(
		filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE),
	);

	const rangeStart = $derived(
		filtered.length === 0 ? 0 : (page - 1) * PAGE_SIZE + 1,
	);

	const rangeEnd = $derived(Math.min(page * PAGE_SIZE, filtered.length));

	$effect(() => {
		query;
		page = 1;
	});

	$effect(() => {
		if (page > totalPages) page = totalPages;
	});

	onMount(() => {
		void store.refreshProjects();
	});

	async function confirmDelete() {
		if (!deleteTarget || deleting) return;
		deleting = true;
		try {
			await store.removeProject(deleteTarget.id);
			deleteTarget = null;
		} finally {
			deleting = false;
		}
	}
</script>

<div class="h-full overflow-y-auto">
	{#if store.projectsLoading && store.projects.length === 0}
		<LibrarySkeleton />
	{:else}
	<div class="mx-auto w-full max-w-[960px] px-8 py-12">
		{#if store.projects.length === 0}
			<section class="flex flex-col items-center px-6 py-20 text-center">
				<Logo markClass="size-14" />
				<h1 class="mt-6 text-2xl font-semibold tracking-tight text-(--text)">
					Document what you just did
				</h1>
				<p class="mt-2 max-w-md text-sm leading-relaxed text-(--text)/56">
					Walkmark watches a screen or window, captures each meaningful step, and writes
					the instructions for you.
				</p>
				<Button class="mt-8" onclick={() => store.setDialog("sources")}>
					<Icon icon="lucide:circle-dot" class="size-4" />
					Record your first guide
				</Button>
				<Button class="mt-3" variant="ghost" onclick={() => void store.importArchive()}>
					<Icon icon="lucide:upload" class="size-4" />
					Import an archive
				</Button>
			</section>
			{@render notices("mt-6")}
		{:else}
			{@render notices("mb-8")}
			<section>
				<div class="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
					<div class="flex items-center gap-3">
						<h2 class="text-xs font-medium uppercase tracking-wider text-(--text)/40">
							Your documents
						</h2>
						<Button size="sm" variant="ghost" onclick={() => void store.importArchive()}>
							<Icon icon="lucide:upload" class="size-3.5" />
							Import
						</Button>
					</div>
					<div class="flex w-full flex-col gap-2 sm:max-w-sm sm:flex-row sm:items-center">
						<div class="relative min-w-0 flex-1">
							<Icon
								icon="lucide:search"
								class="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-(--text)/40"
							/>
							<Input
								bind:value={query}
								maxlength={LIMITS.searchQuery}
								placeholder="Search documents"
								class="pl-9"
							/>
						</div>
						{#if query.trim()}
							<Button
								variant="ghost"
								size="sm"
								class="shrink-0"
								onclick={() => (query = "")}
							>
								Clear
							</Button>
						{/if}
						<div
							class="flex shrink-0 items-center gap-0.5 rounded-xl bg-(--bg-elevated) p-0.5"
							role="group"
							aria-label="View"
						>
							{#each [{ id: "cards", icon: "lucide:layout-grid", label: "Cards" }, { id: "details", icon: "lucide:list", label: "Details" }] as option (option.id)}
								<button
									type="button"
									class="grid size-8 place-items-center rounded-[10px] transition-colors {view === option.id
										? 'bg-(--text)/10 text-(--text)'
										: 'text-(--text)/56 hover:text-(--text)'}"
									aria-label={option.label}
									aria-pressed={view === option.id}
									data-tooltip={option.label}
									onclick={() => setView(option.id as View)}
								>
									<Icon icon={option.icon} class="size-4" />
								</button>
							{/each}
						</div>
					</div>
				</div>

				{#if filtered.length === 0}
					<div class="rounded-3xl bg-(--bg-elevated) px-6 py-14 text-center">
						<Icon icon="lucide:search-x" class="mx-auto mb-3 size-6 text-(--text)/32" />
						<p class="text-sm font-medium text-(--text)">No documents match your search</p>
						<p class="mt-1 text-xs text-(--text)/56">
							Nothing titled like “{query.trim()}”. Try a different phrase.
						</p>
						<Button variant="ghost" size="sm" class="mt-4" onclick={() => (query = "")}>
							Clear search
						</Button>
					</div>
				{:else}
					<p class="mb-4 text-xs text-(--text)/56">
						{filtered.length === store.projects.length
							? pluralize(filtered.length, "document")
							: `${filtered.length} of ${pluralize(store.projects.length, "document")}`}
						{#if totalPages > 1}
							<span aria-hidden="true"> · </span>
							<span>Page {page} of {totalPages}</span>
						{/if}
					</p>

					{#if view === "details"}
						<div class="overflow-hidden rounded-3xl bg-(--bg-elevated)">
							<div
								class="grid grid-cols-[minmax(0,1fr)_5rem_11rem_4.5rem] items-center gap-4 border-b border-(--text)/8 px-5 py-2.5 text-xs font-medium text-(--text)/40"
							>
								<span>Name</span>
								<span>Steps</span>
								<span>Written</span>
								<span class="sr-only">Actions</span>
							</div>
							{#each pageItems as project (project.id)}
								{@render row(project)}
							{/each}
						</div>
					{:else}
						<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
							{#each pageItems as project (project.id)}
								{@render card(project)}
							{/each}
						</div>
					{/if}

					{#if totalPages > 1}
						<div
							class="mt-8 flex flex-col items-center justify-between gap-3 border-t border-(--text)/8 pt-6 sm:flex-row"
						>
							<p class="text-xs text-(--text)/56">
								Showing {rangeStart}–{rangeEnd} of {filtered.length}
							</p>
							<div class="flex items-center gap-2">
								<Button
									variant="ghost"
									size="sm"
									disabled={page <= 1}
									onclick={() => (page -= 1)}
								>
									<Icon icon="lucide:chevron-left" class="size-4" />
									Previous
								</Button>
								<span class="min-w-[5.5rem] text-center text-xs tabular-nums text-(--text)/56">
									{page} / {totalPages}
								</span>
								<Button
									variant="ghost"
									size="sm"
									disabled={page >= totalPages}
									onclick={() => (page += 1)}
								>
									Next
									<Icon icon="lucide:chevron-right" class="size-4" />
								</Button>
							</div>
						</div>
					{/if}
				{/if}
			</section>
		{/if}
	</div>
	{/if}
</div>

<Dialog open={deleteTarget !== null} onOpenChange={(open) => !open && (deleteTarget = null)}>
	<DialogContent class="max-w-md">
		<DialogHeader>
			<DialogTitle>Delete this document?</DialogTitle>
			<DialogDescription>
				{#if deleteTarget}
					“{deleteTarget.title}” and all of its screenshots will be removed permanently.
				{/if}
			</DialogDescription>
		</DialogHeader>
		<DialogFooter>
			<Button variant="ghost" disabled={deleting} onclick={() => (deleteTarget = null)}>
				Cancel
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={() => void confirmDelete()}>
				{deleting ? "Deleting…" : "Delete"}
			</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

{#snippet notices(className: string)}
	{#if store.blockedReason || (!store.permission.granted && store.permission.required)}
		<div class="flex flex-col gap-2 {className}">
			{#if !store.permission.granted && store.permission.required}
				<div class="flex items-center gap-3 rounded-3xl bg-(--bg-elevated) px-4 py-3">
					<Icon icon="lucide:monitor-play" class="size-4 flex-none text-red-500" />
					<div class="min-w-0 flex-1">
						<p class="text-sm font-medium text-(--text)">Screen recording access is off</p>
						<p class="text-xs text-(--text)/56">macOS needs permission before Walkmark can capture.</p>
					</div>
					<Button size="sm" variant="ghost" onclick={() => store.setDialog("onboarding")}>
						Fix this
					</Button>
				</div>
			{/if}
			{#if store.blockedReason}
				<div class="flex items-center gap-3 rounded-3xl bg-(--bg-elevated) px-4 py-3">
					<Icon icon="lucide:key-round" class="size-4 flex-none text-(--text)/56" />
					<div class="min-w-0 flex-1">
						<p class="text-sm font-medium text-(--text)">Select an AI model</p>
						<p class="text-xs text-(--text)/56">{store.blockedReason}</p>
					</div>
					<Button size="sm" variant="ghost" onclick={() => store.setDialog("settings")}>
						Set up
					</Button>
				</div>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet card(project: ProjectSummary)}
	{@const done = project.stepCount > 0 && project.readyCount === project.stepCount}
	<article
		class="group relative overflow-hidden rounded-3xl bg-(--bg-elevated) transition-transform hover:-translate-y-0.5"
	>
		<button
			type="button"
			class="block w-full text-left"
			onclick={() => void store.openProject(project.id)}
		>
			<div class="relative aspect-video overflow-hidden bg-(--bg)">
				{#if project.cover}
					<img
						src={convertFileSrc(project.cover)}
						alt=""
						draggable="false"
						class="size-full object-cover object-top transition-transform duration-500 group-hover:scale-[1.02]"
						loading="lazy"
					/>
				{:else}
					<div class="grid size-full place-items-center text-(--text)/24">
						<Icon icon="lucide:file-text" class="size-6" />
					</div>
				{/if}
			</div>
			<div class="p-4">
				<h3 class="truncate text-sm font-medium text-(--text)">{project.title}</h3>
				<p class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-(--text)/56">
					<span>{pluralize(project.stepCount, "step")}</span>
					<span>·</span>
					<span>{relativeTime(project.updatedAt)}</span>
					{#if !done && project.stepCount > 0}
						<span>·</span>
						<Badge>{project.stepCount - project.readyCount} unwritten</Badge>
					{/if}
				</p>
			</div>
		</button>

		<div class="absolute top-3 right-3 z-10 flex items-center gap-1.5 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
			{@render actions(project)}
		</div>
	</article>
{/snippet}

{#snippet actions(project: ProjectSummary)}
	<button
		type="button"
		class="grid size-8 place-items-center rounded-xl bg-(--bg-elevated)/95 text-(--text)/72 shadow-sm ring-1 ring-(--text)/10 backdrop-blur-sm transition-colors hover:bg-(--bg-elevated) hover:text-(--text) disabled:opacity-40"
		aria-label="Export {project.title}" data-tooltip="Export"
		disabled={project.stepCount === 0}
		onclick={(event) => {
			event.stopPropagation();
			void store.exportFromLibrary(project.id);
		}}
	>
		<Icon icon="lucide:download" class="size-4" />
	</button>
	<DropdownMenu>
		<DropdownMenuTrigger>
			{#snippet child({ props })}
				<button
					{...props}
					type="button"
					class="grid size-8 place-items-center rounded-xl bg-(--bg-elevated)/95 text-(--text)/72 shadow-sm ring-1 ring-(--text)/10 backdrop-blur-sm transition-colors hover:bg-(--bg-elevated) hover:text-(--text)"
					aria-label="Document options for {project.title}" data-tooltip="Document options"
					onclick={(event) => event.stopPropagation()}
				>
					<Icon icon="lucide:ellipsis" class="size-4" />
				</button>
			{/snippet}
		</DropdownMenuTrigger>
		<DropdownMenuContent align="end" class="w-40">
			<DropdownMenuItem
				disabled={project.stepCount === 0}
				onclick={() => void store.exportFromLibrary(project.id)}
			>
				<Icon icon="lucide:download" />
				Export…
			</DropdownMenuItem>
			<DropdownMenuItem onclick={() => void store.exportArchive(project)}>
				<Icon icon="lucide:download" />
				Export archive (.zip)
			</DropdownMenuItem>
			<DropdownMenuItem
				variant="destructive"
				onclick={() => (deleteTarget = project)}
			>
				<Icon icon="lucide:trash-2" />
				Delete
			</DropdownMenuItem>
		</DropdownMenuContent>
	</DropdownMenu>
{/snippet}

{#snippet row(project: ProjectSummary)}
	<article
		class="group relative border-b border-(--text)/6 last:border-b-0 hover:bg-(--text)/4"
	>
		<button
			type="button"
			class="grid w-full grid-cols-[minmax(0,1fr)_5rem_11rem_4.5rem] items-center gap-4 px-5 py-3 text-left"
			onclick={() => void store.openProject(project.id)}
		>
			<span class="truncate text-sm font-medium text-(--text)">{project.title}</span>
			<span class="text-xs tabular-nums text-(--text)/56">{project.stepCount}</span>
			<span class="truncate text-xs text-(--text)/56">{writtenOn(project.updatedAt)}</span>
			<span></span>
		</button>
		<div
			class="absolute top-1/2 right-3 z-10 flex -translate-y-1/2 items-center gap-1.5 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
		>
			{@render actions(project)}
		</div>
	</article>
{/snippet}
