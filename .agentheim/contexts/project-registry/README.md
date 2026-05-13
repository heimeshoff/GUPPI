# project-registry

## Purpose

Discovers, lists, and creates Agentheim projects on disk. Watches the filesystem for folders containing an `.agentheim/` directory, enumerates each project's bounded contexts (`contexts/<name>/`), counts tasks per state (backlog / todo / doing / done) per BC, and exposes this model upstream to the canvas. Also owns the new-project flow: create folder, `git init`, invoke the `brainstorm` skill inside the new folder (which is itself a `claude-runner` operation).

In v1 this BC is read-only-plus-create: it observes existing projects and creates new ones, but does not mutate the internal state of existing projects (that's `claude-runner`'s job, via spawned `claude` sessions).

## Classification

**Supporting.** Necessary scaffolding — without it the canvas has nothing to draw — but discovery itself is not GUPPI's differentiator. The Agentheim-on-disk shape is specific enough that this isn't pure generic plumbing, but the value-add is in what the canvas and `agent-awareness` do with the discovered projects.

## Ubiquitous language (seed)

- **Project** — a folder on disk containing an `.agentheim/` directory.
- **Agentheim project** — synonym for project, used when disambiguating from arbitrary folders.
- **Bounded context (BC)** — a `contexts/<name>/` directory inside a project.
- **Task** — a markdown file under `contexts/<name>/{backlog,todo,doing,done}/`.
- **Task state** — backlog / todo / doing / done, derived from which subdirectory the task file lives in.
- **Discovery** — the act of scanning the filesystem for Agentheim projects.
- **New-project flow** — the sequence: create folder → `git init` → invoke `brainstorm`.
- **Vision file** — `.agentheim/vision.md`, the canonical "this project exists" marker (alongside the `.agentheim/` directory itself).

## Upstream / downstream

- **Downstream of:** the filesystem (conformist to the Agentheim-on-disk shape — if the shape changes upstream, the fix lives here).
- **Upstream of:** `canvas` (supplies the project model), `agent-awareness` (may share the filesystem watcher — foundation decision).

## Open questions

- Is the filesystem watcher shared with `agent-awareness` (one watcher, two consumers via the infrastructure event bus) or independent? Foundation pass.
- Where does GUPPI look for projects? Configured roots? Recent-projects list? User-added? Foundation/UX decision.
